//! Implements a simpler version of I18n.
//! Supports 2 built-in data providers (for static projects where the files do not change and where the localization file can be changed by the user or developer).
//! For other cases, you can write your own data provider.
//! [Github project](https://github.com/SinmoWay/simple-i18n)
//! [Crates](https://crates.io/crates/sorrow-i18n)

#![deny(missing_docs)]
#![deny(warnings)]

/// Macro feature.
/// Adds 2 macros, the first one serves for initialization, the second one for getting the value from the holders.
///
/// # Examples
///
/// ```
/// // Let's initialize our i18n core.
/// init_i18n!("locale/");
///
/// // Getting data by holder. (locale is required)
/// // If the key is not found or the locale is not found, return the passed key.
/// let test = i18n!("RU", "data.name");
/// assert_eq!("Тест", &*test);
/// let not_found_data = i18n!("RU", "data.not_found_me");
/// assert_eq!("data.not_found_me", &*not_found_data);
/// ```
#[cfg(feature = "macro")]
pub mod feature_macro;

use std::collections::HashMap;
use std::fs::{File};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::sleep;
use std::time::Duration;
use sys_locale::get_locale;

use err_derive::Error;
#[cfg(feature = "incl_dir")]
use include_dir::Dir;
use notify::{ErrorKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde_yaml::Value;

/// Error type
pub type Error = I18nError;

/// Library errors
#[derive(Debug, Error)]
pub enum I18nError {
    /// Access denied for file.
    /// Not found file and e.t.c.
    #[error(display = "File with path {:?} not found.", path)]
    IoError {
        /// The file that generated the error
        path: String,
        /// Cause message
        cause: String,
    },

    /// Invalid structure locale file.
    #[error(display = "Structure with path {:?} invalid. Additional information: {:?}", path, cause)]
    InvalidStructure {
        /// The file that generated the error
        path: String,
        /// Cause message
        cause: String,
    },

    /// Invalid kind of file.
    #[error(display = "Structure with path {:?} invalid. Expected kind: I18N.", path)]
    InvalidHeader {
        /// The file that generated the error
        path: String,
    },

    /// Error while watching by file.
    #[error(display = "Watching by file return error: {:?}", message)]
    WatchError {
        /// Cause message
        message: String
    },

    /// File extension is not .yaml or .yml
    #[error(display = "File type is not supported: {:?}", path)]
    NotSupportedFileExtension {
        /// The file that generated the error
        path: String
    },

    /// The error is generated when you have two files with the same locale or when you manually add an existing locale.
    #[error(display = "Duplicate locale holder for {:?}", locale)]
    DuplicateLocale {
        /// Duplicate locale
        locale: String
    },

    /// An error due to which the provider was not added to the locale.
    #[error(display = "The provider has not been added to the {:?} locale. Cause: {:?}", locale, cause)]
    ProviderNotAddedError {
        /// Locale where provider return error
        locale: String,
        /// Cause error
        cause: String,
    },
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
/// use sorrow_i18n::{Error, WatchProvider};
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
///     use sorrow_i18n::InternationalCore;
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Providers {
    /// [FileProvider] - dynamically watcher for file.
    FileProvider,
    /// [StaticFileProvider] - static file. It is not being watched. Default option if the `provider` is not specified in the file structure
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
                // Validation file
                let structure = load_struct(&path.clone()).unwrap();
                // Lock data and clear
                let mut w_holder = holder.write().unwrap();
                w_holder.clear();

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
                        Err(Error::IoError { path: self.path.clone(), cause: String::default() })
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
///
/// # Examples
///
/// ```
/// use include_dir::Dir;
/// use sorrow_i18n::InternationalCore;
/// const PROJECT_DIR: Dir = include_dir!("resources/en_ru");
/// fn main() {
///     let core = InternationalCore::from(PROJECT_DIR);
///     let locale_holder = core.get_by_locale("my_locale").unwrap();
/// }
/// ```
#[cfg(feature = "incl_dir")]
impl<'a> From<Dir<'a>> for InternationalCore {
    fn from(dir: Dir) -> Self {
        let files = dir.files();
        let mut msg_holder = HashMap::new();
        // Folder is not required if files include in project.
        // Setting default watcher by StaticFileProvider immediately.
        for file in files {
            let content = std::str::from_utf8(file.contents()).unwrap();
            let structure = load_struct_from_str(content, None).unwrap();
            let cl_struct = Arc::clone(&structure.provider);
            let provider = cl_struct.lock();
            match provider {
                Ok(mut provider) => {
                    *provider = Box::new(StaticFileProvider {});
                }
                Err(_e) => {
                    panic!("Update provider by file has been failed. Poison mutex status.");
                }
            }
            msg_holder.insert(structure.locale.clone(), structure);
        };
        InternationalCore {
            holders: msg_holder
        }
    }
}

impl InternationalCore {
    /// Creating new instance of InternationalCore.
    ///
    /// # Example
    /// ```
    /// use sorrow_i18n::InternationalCore;
    /// let core = InternationalCore::new("folder/locales");
    /// ```
    /// If the file generates an error [Error::NotSupportedFileExtension], it will be skipped.
    /// The rest of the errors cause panic.
    pub fn new<S: Into<String>>(folder: S) -> InternationalCore {
        let folder = folder.into();
        let dir = std::fs::read_dir(&folder)
            .map_err(|e| {
                log::error!("{}", &e);
                Error::IoError { path: folder, cause: e.to_string() }
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
    pub fn add_provider(&mut self, locale: &str, provider: Box<dyn WatchProvider + 'static + Sync + Send>) -> Result<(), Error> {
        let holder = self.holders.get(locale);
        match holder {
            None => {
                log::warn!("The provider has not been added. The locale to which you tried to add the provider does not exist.");
                return Err(Error::ProviderNotAddedError { locale: locale.to_string(), cause: "locale not found.".to_string() });
            }
            Some(holder) => {
                let guard = holder.provider.lock();
                match guard {
                    Ok(mut pr) => {
                        *pr = provider;
                        pr.set_data(Arc::clone(&holder.messages))?;
                        pr.watch()?;
                    }
                    Err(_e) => {
                        log::error!("Failed to update provider. Mutex on provider has been poison.");
                        panic!("Poison mutex on add provider.");
                    }
                }
            }
        }
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
    /// Getting locale message by key. If key does not exist, return [Option::None].
    ///
    /// # Examples
    ///
    /// ```
    /// use sorrow_i18n::{GetData, InternationalCore};
    /// let i18n = InternationalCore::new("locale");
    /// let en = i18n.get_by_locale("en").unwrap();
    /// let my_data = en.get("my_data");
    ///
    /// match my_data {
    ///     None => {
    ///         panic!("No found my_data key.")
    ///     }
    ///     Some(k) => {
    ///         println!("Found key my_data, value: {}", &k)
    ///     }
    /// }
    /// ```
    fn get<S: AsRef<str>>(&self, key: S) -> Option<String>;

    /// Getting locale message by key. If key does not exist, return `key`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sorrow_i18n::{GetData, InternationalCore};
    /// let i18n = InternationalCore::new("locale");
    /// let en = i18n.get_by_locale("en").unwrap();
    /// // If data is not found ref my_data == "my_data"
    /// // Else you getting data.
    /// let my_data = en.get_or_default("my_data");
    /// ```
    fn get_or_default<S: AsRef<str>>(&self, key: S) -> String;

    /// Getting all keys in holder's
    ///
    /// # Examples
    ///
    /// ```
    /// use sorrow_i18n::{GetData, InternationalCore};
    /// let i18n = InternationalCore::new("locale");
    /// let en = i18n.get_by_locale("en").unwrap();
    /// let keys = en.keys();
    /// // Print all key's in en locale holder.
    /// keys.iter().for_each(|k| println!("{}", k));
    /// ```
    fn keys(&self) -> Vec<String>;
}

/// Works with an ordinary hash map, useful when the data never changes.
/// It's simple wrapper.
pub struct UnWatchData {
    holder: HashMap<String, String>,
}

impl UnWatchData {
    /// Creating [UnWatchData] by reference for original data.
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
        return self.get(key.as_ref()).unwrap_or_else(|| key.as_ref().to_string());
    }

    fn keys(&self) -> Vec<String> {
        self.holder.keys().map(|k| k.to_string()).collect::<Vec<String>>()
    }
}

/// We work with a mutable data ref.
pub struct Data {
    holder: Arc<RwLock<HashMap<String, String>>>,
}

impl Data {
    /// Creating [Data] by reference for original data. (mutable)
    pub fn new(holder: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Data {
            holder: Arc::clone(&holder)
        }
    }
}

impl GetData for Data {
    fn get<S: AsRef<str>>(&self, key: S) -> Option<String> {
        let state = self.holder.read().unwrap();
        return state.get(key.as_ref()).map(|r| r.to_string());
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

    fn keys(&self) -> Vec<String> {
        self.holder.read().unwrap().keys().map(|k| k.to_string()).collect::<Vec<String>>()
    }
}

/// The simplest information keeper.
/// Contains a link to the data itself that can be dynamically updated.
/// The locale for determining what this state refers to.
/// And also, the provider who is responsible for the volatility of the data.
pub struct Holder {
    messages: Arc<RwLock<HashMap<String, String>>>,
    locale: String,
    provider: Arc<Mutex<Box<dyn WatchProvider + Sync + Send>>>,
}

impl Holder {
    /// Return [Holder]
    ///
    /// # Arguments
    ///
    /// * `path` - folder by localization's.
    ///
    /// # Examples
    /// ```
    /// use sorrow_i18n::Holder;
    /// let holder = Holder::new("my_locale_folder");
    /// ```
    pub fn new<S: Into<String>>(path: S) -> Result<Holder, Error> {
        load_struct(path)
    }
}

impl WatchProvider for Holder {
    fn watch(&mut self) -> Result<(), Error> {
        self.provider.lock().unwrap().watch()
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.messages = data;
        Ok(())
    }
}

enum FileData {
    Map(HashMap<String, FileData>),
    String(String),
}

/// Default structure by file localization.
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
    /// Kind - I18N. It is necessary to understand if the file does not belong to the localization category.
    kind: String,

    /// Locale - file locale type.
    locale: String,

    /// Description - for user, optional parameter.
    description: Option<String>,

    /// Provider - optional parameter, if is None, [StaticFileProvider]. For additional information see [Providers].
    provider: Option<Providers>,

    /// Data - localization information. Format key-value, optional.
    #[serde(flatten)]
    data: Option<Value>,
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

    let locale = structure.locale.clone();

    match structure.data {
        None => {
            log::warn!("Empty data for {} locale. File path: {}.", &structure.locale, &*path);
        }
        Some(kv) => {
            messages
                .write()
                .and_then(|mut m| {
                    m.extend(to_flatten(String::default(), FileData::from(kv)));
                    Ok(())
                }).unwrap();
        }
    };

    return match structure.provider {
        None => {
            // Unwatch if provider is not exists
            Ok(Holder {
                messages,
                locale,
                provider: Arc::new(Mutex::new(Box::new(StaticFileProvider {}))),
            })
        }
        Some(p) => {
            match p {
                Providers::FileProvider => {
                    let provider = FileProvider::new(Arc::clone(&messages), path.clone());
                    Ok(Holder {
                        messages,
                        locale,
                        provider: Arc::new(Mutex::new(Box::new(provider))),
                    })
                }
                Providers::StaticFileProvider => {
                    Ok(Holder {
                        messages,
                        locale,
                        provider: Arc::new(Mutex::new(Box::new(StaticFileProvider {}))),
                    })
                }
            }
        }
    };
}

/// Load file ant trigger loading [FileStructure] by [load_struct_from_str()]
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
            log::error!("Error while open file {}. Additional information: {}", &path, &e);
            Error::IoError {
                path: path.clone(),
                cause: e.to_string(),
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

impl From<serde_yaml::Value> for FileData {
    fn from(value: serde_yaml::Value) -> Self {
        match value {
            serde_yaml::Value::Mapping(obj) => FileData::Map(
                obj.into_iter()
                    .filter_map(|(k, v)| match k {
                        serde_yaml::Value::String(s) => Some((s, FileData::from(v))),
                        _ => None,
                    })
                    .collect(),
            ),
            serde_yaml::Value::String(s) => FileData::String(s),
            _ => FileData::Map(Default::default()),
        }
    }
}

fn to_flatten(name: String, val: FileData) -> HashMap<String, String> {
    let mut map = HashMap::new();
    match val {
        FileData::Map(array) => {
            for (name2, v) in array.into_iter() {
                map.extend(to_flatten(
                    if name.is_empty() {
                        name2
                    } else {
                        format!("{}.{}", name, name2)
                    },
                    v,
                ));
            }
        }
        FileData::String(s) => {
            map.insert(name, s.clone());
        }
    };
    map
}