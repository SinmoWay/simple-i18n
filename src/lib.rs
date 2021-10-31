//! Implements a simpler version of I18n.
//! Supports 2 built-in data providers (for static projects where the files do not change and where the localization file can be changed by the user or developer).
//! For other cases, you can write your own data provider.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use sys_locale::get_locale;

use err_derive::Error;
#[cfg(feature = "incl_dir")]
use include_dir::Dir;
use notify::{ErrorKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Error type
pub type Error = I18nError;

#[derive(Debug, Error)]
pub enum I18nError {
    /// Access denied for file.
    /// Not found file and e.t.c.
    #[error(display = "File with path {:?} not found.", path)]
    IoError { path: String },

    /// Invalid structure locale file.
    #[error(display = "Structure with path {:?} invalid. Additional information: {:?}", path, cause)]
    InvalidStructure { path: String, cause: String },

    /// Invalid kind of file.
    #[error(display = "Structure with path {:?} invalid. Expected kind: I18N.", path)]
    InvalidHeader { path: String },

    /// Error while watching by file.
    #[error(display = "Watching by file return error: {:?}", message)]
    WatchError { message: String },

    /// File extension is not .yaml or .yml
    #[error(display = "File type is not supported: {:?}", path)]
    NotSupportedFileExtension { path: String },

    /// The error is generated when you have two files with the same locale or when you manually add an existing locale.
    #[error(display = "Duplicate locale holder for {:?}", locale)]
    DuplicateLocale { locale: String },
}

/// Implementation of the state observer.
/// You can implement your own observer and add it to the MessageHolder, for this see an example in examples/custom_provider.rs
///
/// # Implementation
///
/// The library provides 2 types of observers:
/// 1) For static files (does not imply changing them) [StaticFileProvider]
/// 2) For files that you plan to modify in any way. Or do you mean such a possibility. [FileProvider]
///
/// # Examples for custom provider
/// Creating base structure for provider.
/// The provider must work with information, which means he must receive a link to the working data.
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::{Arc, RwLock};
///
/// pub struct CustomProvider {
///     data: Arc<RwLock<HashMap<String, String>>>,
/// }
/// ```
/// ## Implementation WatchProvider for provider
///
///
/// ```
/// use std::collections::HashMap;
/// use std::sync::{Arc, RwLock};
/// use simple_i18n::{Error, WatchProvider};
///
/// impl WatchProvider for CustomProvider {
///     fn watch(&mut self) -> Result<(), Error> {
///         println!("Accepted custom provider");
///         Ok(())
///     }
///
/// // Setting current data in holder.
///     fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
///         self.data = data;
///         println!("Data has been set");
///         Ok(())
///     }
/// }
///
/// ```
///
/// # Using in project
///
/// Add provider for holder.
///
/// ```
///     use simple_i18n::InternationalCore;
///
///     let mut core = InternationalCore::new("resources/locales");
///     core.add_provider("my locale", Box::new(CustomProvider::new())).unwrap();
/// ```
///
pub trait WatchProvider {
    /// The main observer method that is called to observe the state.
    fn watch(&mut self) -> Result<(), Error>;

    /// Setter for data reference.
    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error>;
}

/// Base providers
/// [FileProvider] - dynamically watcher for file.
/// [StaticFileProvider] - static file. It is not being watched. Default option if the `provider` is not specified in the file structure
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Providers {
    FileProvider,
    StaticFileProvider,
}

/// Files maybe changed. Watch by `modify` system event.
struct FileProvider {
    messages: Arc<RwLock<HashMap<String, String>>>,
    path: String,
    watcher: Option<RecommendedWatcher>,
}

impl FileProvider {
    pub fn new(messages: Arc<RwLock<HashMap<String, String>>>, path: String) -> Self {
        FileProvider {
            messages,
            path,
            watcher: None,
        }
    }
}

impl WatchProvider for FileProvider {
    fn watch(&mut self) -> Result<(), Error> {
        let holder = Arc::clone(&self.messages);
        let path = self.path.clone();
        let res_watcher = notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
            let event = result.map_err(|e| Error::WatchError { message: e.to_string() }).unwrap();
            if event.kind.is_modify() {
                // Hack.
                // Inappropriate library behavior was detected when the file was updated on the Winodws platform.
                // For some reason, 2 save events are fired, which causes double reads of the file.
                // At the same time, the intervals between reading the file (updated configuration) are too small, which causes an error in the form of EOF.
                // The simplest solution is to set a minimum timeout between these events.
                sleep(Duration::from_millis(10));
                log::debug!("Modify {}. Reloading data.", &path.clone());
                // Lock data and clear
                let mut w_holder = holder.write().unwrap();
                w_holder.clear();

                // Validation file
                let structure = load_struct(&path.clone()).unwrap();
                // Clone internal state.
                let l_holder = structure.messages.write().unwrap().clone();
                w_holder.extend(l_holder);
                // Unlock
            }
        });

        return match res_watcher {
            Ok(mut w) => {
                // TODO: Check error's?
                w.watch(Path::new(&self.path.clone()), RecursiveMode::NonRecursive).unwrap();
                self.watcher = Some(w);
                Ok(())
            }

            Err(e) => {
                match e.kind {
                    ErrorKind::Generic(message) => {
                        log::error!("Error while watch by file {}. Message: {}", &self.path, &message);
                        Err(Error::WatchError { message })
                    }
                    ErrorKind::Io(err) => {
                        log::error!("Error while watch by file {}. Message: {}", &self.path, &err);
                        Err(Error::WatchError { message: err.to_string() })
                    }
                    ErrorKind::PathNotFound => {
                        log::error!("Path not found: {}", &self.path);
                        Err(Error::IoError { path: self.path.clone() })
                    }
                    ErrorKind::WatchNotFound => {
                        log::error!("Watcher not found for: {}", &self.path);
                        // Ignore
                        Ok(())
                    }
                    ErrorKind::InvalidConfig(err) => {
                        log::error!("Invalid watch config. {:?}", &err);
                        // Ignore
                        Ok(())
                    }
                    ErrorKind::MaxFilesWatch => {
                        log::error!("Max watchers for: {}", &self.path);
                        Err(Error::WatchError { message: String::from("max watchers for file. Please try again latter or remove exists watcher.") })
                    }
                }
            }
        };
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.messages = data;
        Ok(())
    }
}

/// Files does not changed. Only loading files.
/// Default option by [FileStructure]
struct StaticFileProvider {}

impl WatchProvider for StaticFileProvider {
    fn watch(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn set_data(&mut self, _data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        Ok(())
    }
}

/// Holder for localization map.
pub struct InternationalCore {
    holders: HashMap<String, Holder>,
}

/// Additional library, use features = ["incl_dir"] to enable.
/// Helps to include static files in the project that will not change.
/// See for example 'eu_ru_localization_incl_dir.rs'
#[cfg(feature = "incl_dir")]
impl<'a> From<Dir<'a>> for InternationalCore {
    fn from(dir: Dir) -> Self {
        let files = dir.files();
        let mut msg_holder = HashMap::new();
        // Folder is not required if files include in project.
        // Setting default watcher by StaticFileProvider immediately.
        for file in files {
            let content = std::str::from_utf8(file.contents()).unwrap();
            let mut structure = load_struct_from_str(content, None).unwrap();
            structure.provider = RefCell::new(Box::new(StaticFileProvider {}));
            msg_holder.insert(structure.locale.clone(), structure);
        };
        InternationalCore {
            holders: msg_holder
        }
    }
}

impl InternationalCore {
    pub fn new<S: Into<String>>(folder: S) -> InternationalCore {
        let folder = folder.into();
        let dir = std::fs::read_dir(&folder)
            .map_err(|e| {
                log::error!("{}", e);
                Error::IoError { path: folder }
            }).unwrap();
        let mut msg_holder = HashMap::new();

        for path in dir {
            let full_path = path.unwrap().path().to_str().unwrap().to_string();
            let holder = Holder::new(full_path);
            match holder {
                Ok(mut holder) => {
                    holder.watch().unwrap();
                    msg_holder.insert(holder.locale.clone(), holder);
                }
                Err(err) => {
                    match err {
                        Error::NotSupportedFileExtension { path } => {
                            log::trace!("Skipped {}, file is not supported .yml/.yaml extension.", path);
                            continue;
                        }
                        e => {
                            panic!("Error while loading file. {:?}", e)
                        }
                    }
                }
            }
        }
        InternationalCore { holders: msg_holder }
    }

    /// Get a mutable link to your localization. If no localization is found, you will get `None`.
    pub fn get_by_locale(&self, locale: &str) -> Option<Data> {
        let holders = &self.holders;
        let holder = holders.get(locale)?;
        Some(Data::new(Arc::clone(&holder.messages)))
    }

    /// Get a mutable link to your system localization. If no localization is found, you will get `None`.
    pub fn get_current_locale(&self) -> Option<Data> {
        let locale = get_current_locale_or_default();
        self.get_by_locale(&*locale)
    }

    /// Get unmodifiable values (UnWatch). Perfect for localizations built into the project, due to which you get a small wrapper on `HashMap`.
    /// If no localization is found, you will get `None`.
    pub fn get_by_locale_state(&self, locale: &str) -> Option<UnWatchData> {
        let holders = &self.holders;
        let holder = holders.get(locale)?;
        let read_state = holder.messages.read().unwrap();
        Some(UnWatchData::new(&read_state))
    }

    /// Get unmodifiable values (UnWatch). Perfect for localizations built into the project, due to which you get a small wrapper on `HashMap`.
    /// If no localization is found, you will get `None`. If a localization is found, then returns the current system localization.
    pub fn get_current_locale_state(&self) -> Option<UnWatchData> {
        let locale = get_current_locale_or_default();
        let state = self.get_by_locale_state(&*locale)?;
        Some(state)
    }

    /// Overrides the current provider for your localization. Implementation example: `examples/custom_provider.rs`
    pub fn add_provider(&mut self, locale: &str, provider: Box<dyn WatchProvider + 'static>) -> Result<(), Error> {
        let holder = self.holders.get(locale);
        let holder = holder.unwrap();
        holder.provider.replace(provider);
        holder.provider.borrow_mut().set_data(Arc::clone(&holder.messages))?;
        holder.provider.borrow_mut().watch()?;
        Ok(())
    }

    /// Add locale with custom locale holder
    pub fn add_locale(&mut self, locale: &str, locale_holder: Holder) -> Result<(), Error> {
        let holder = self.holders.get(locale);
        return if holder.is_some() {
            Err(Error::DuplicateLocale { locale: locale.to_string() })
        } else {
            self.holders.insert(locale.to_string(), locale_holder);
            Ok(())
        };
    }
}

/// Getting data by holder's.
pub trait GetData {
    fn get<S: AsRef<str>>(&self, key: S) -> Option<String>;
    fn get_or_default<S: AsRef<str>>(&self, key: S) -> String;
}

pub struct UnWatchData {
    holder: HashMap<String, String>,
}

impl UnWatchData {
    pub fn new(holder: &HashMap<String, String>) -> Self {
        UnWatchData {
            holder: holder.clone()
        }
    }
}

impl GetData for UnWatchData {
    fn get<S: AsRef<str>>(&self, key: S) -> Option<String> {
        return self.holder.get(key.as_ref()).map(|r| r.to_string());
    }

    fn get_or_default<S: AsRef<str>>(&self, key: S) -> String {
        return match self.holder.get(key.as_ref()) {
            None => {
                key.as_ref().to_string().clone()
            }
            Some(v) => {
                v.clone()
            }
        };
    }
}

pub struct Data {
    holder: Arc<RwLock<HashMap<String, String>>>,
}

impl Data {
    pub fn new(holder: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Data {
            holder: Arc::clone(&holder)
        }
    }
}

impl GetData for Data {
    fn get<S: AsRef<str>>(&self, key: S) -> Option<String> {
        let state = self.holder.read().unwrap();
        return state.clone().get(key.as_ref()).map(|r| r.to_string());
    }

    fn get_or_default<S: AsRef<str>>(&self, key: S) -> String {
        return match self.holder.read().unwrap().get(key.as_ref()) {
            None => {
                key.as_ref().to_string().clone()
            }
            Some(v) => {
                v.clone()
            }
        };
    }
}

/// The simplest information keeper.
/// Contains a link to the data itself that can be dynamically updated.
/// The locale for determining what this state refers to.
/// And also, the provider who is responsible for the volatility of the data.
pub struct Holder {
    messages: Arc<RwLock<HashMap<String, String>>>,
    locale: String,
    provider: RefCell<Box<dyn WatchProvider>>,
}

impl Holder {
    pub fn new<S: Into<String>>(path: S) -> Result<Holder, Error> {
        load_struct(path)
    }
}

impl WatchProvider for Holder {
    fn watch(&mut self) -> Result<(), Error> {
        self.provider.borrow_mut().watch()
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.messages = data;
        Ok(())
    }
}

/// Default structure include:
/// Kind - I18N. It is necessary to understand if the file does not belong to the localization category.
/// Locale - file locale type.
/// Description - for user, optional parameter.
/// Provider - optional parameter, if is None, [StaticFileProvider]. For additional information see [Providers].
/// Data - localization information. Format key-value, optional.
///
/// #Examples
///
/// Basic usage:
///
/// ```yaml
/// kind: I18N
/// locale: EE
/// description: test en
/// data:
///   name: "Helly belly"
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileStructure {
    kind: String,
    locale: String,
    description: Option<String>,
    provider: Option<Providers>,
    data: Option<HashMap<String, String>>,
}

/// Loading [FileStructure], and creating [Holder].
/// If structure is invalid [Error::InvalidStructure]
/// If structure is valid, but kind is not valid, return: [Error::InvalidHeader]
/// Path - optional if use static provider with [incl_dir] `features`.
fn load_struct_from_str(data: &str, path: Option<String>) -> Result<Holder, Error> {
    let messages = Arc::new(RwLock::new(HashMap::new()));
    let path = path.unwrap_or_default();
    let structure: FileStructure = serde_yaml::from_str(data).map_err(|e| Error::InvalidStructure { path: path.clone(), cause: e.to_string() })?;

    if structure.kind.ne("I18N") {
        log::error!("Invalid header for file: {}. Expected: I18N.", &path);
        return Err(Error::InvalidHeader { path: path.clone() });
    };

    log::trace!("Loading structure by path: {}.\nDescription: {:?}\nLocale: {}", &path, &structure.description, &structure.locale);

    let locale = structure.locale;

    match structure.data {
        None => {}
        Some(kv) => {
            messages
                .write()
                .and_then(|mut m| {
                    m.extend(kv);
                    Ok(())
                }).unwrap();
            // Need check this error.
        }
    };

    return match structure.provider {
        None => {
            // Unwatch if provider is not exists
            Ok(Holder {
                messages,
                locale,
                provider: RefCell::new(Box::new(StaticFileProvider {})),
            })
        }
        Some(p) => {
            match p {
                Providers::FileProvider => {
                    let provider = FileProvider::new(Arc::clone(&messages), path.clone());
                    Ok(Holder {
                        messages,
                        locale,
                        provider: RefCell::new(Box::new(provider)),
                    })
                }
                Providers::StaticFileProvider => {
                    Ok(Holder {
                        messages,
                        locale,
                        provider: RefCell::new(Box::new(StaticFileProvider {})),
                    })
                }
            }
        }
    };
}

/// Load file ant trigger loading [FileStructure] by `fn load_struct_from_str`
/// If file extension is not .yaml or .yml, the error is hit [Error::NotSupportedFileExtension]
/// Another error, if IO operation has been failed. [Error::IoError]
fn load_struct<S: Into<String>>(path: S) -> Result<Holder, Error> {
    let mut data = String::new();
    let path = path.into().trim_end().to_string();

    if !path.ends_with(".yaml") && !path.ends_with(".yml") {
        return Err(Error::NotSupportedFileExtension { path: path.clone() });
    }

    let mut file = File::open(&path)
        .map_err(|e| {
            log::error!("Error while open file {}. Additional information: {}", &path, e);
            Error::IoError {
                path: path.clone()
            }
        })?;
    file.read_to_string(&mut data).unwrap();
    load_struct_from_str(&*data, Some(path))
}

/// Getting locale or default by `locale` parameter with `sys-locale` library.
fn get_locale_or_default(locale: &str) -> String {
    get_locale().unwrap_or(String::from(locale))
}

/// Get current system locale or return default `en-US`
fn get_current_locale_or_default() -> String {
    get_locale_or_default("en-US")
}