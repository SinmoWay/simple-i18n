use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File};
use std::io::Read;
use std::sync::{Arc, RwLock};
use sys_locale::get_locale;

use err_derive::Error;
#[cfg(feature = "incl_dir")]
use include_dir::Dir;
use notify::{ErrorKind, RecommendedWatcher};

pub type Error = I18nError;

#[derive(Debug, Error)]
pub enum I18nError {
    #[error(display = "File with path {:?} not found.", path)]
    FileNotFound { path: String },
    #[error(display = "Structure with path {:?} invalid. Additional information: {:?}", path, cause)]
    InvalidStructure { path: String, cause: String },
    #[error(display = "Structure with path {:?} invalid. Expected kind: I18N.", path)]
    InvalidHeader { path: String },
    #[error(display = "Watching by file return error: {:?}", message)]
    WatchError { message: String },
    #[error(display = "File type is not supported: {:?}", path)]
    NotSupportedFileExtension { path: String },
    #[error(display = "Locale {:?} is not found by holder's.", locale)]
    LocaleNotFound { locale: String },
}

pub trait WatchProvider {
    fn watch(&mut self) -> Result<(), Error>;

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error>;
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Providers {
    FileProvider,
    StaticFileProvider,
}

/// Files maybe changed.
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
                log::debug!("Modify {}. Reloading data.", &path);
                // Lock data
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
            Ok(w) => {
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
                        Err(Error::FileNotFound { path: self.path.clone() })
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

/// Files not changed.
/// Only loading files, and not watch by resources.
struct StaticFileProvider {}

impl WatchProvider for StaticFileProvider {
    fn watch(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn set_data(&mut self, _data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        Ok(())
    }
}

pub struct InternationalCore {
    holders: HashMap<String, MessageHolder>,
}

#[cfg(feature = "incl_dir")]
impl<'a> From<Dir<'a>> for InternationalCore {
    fn from(dir: Dir) -> Self {
        let files = dir.files();
        let mut msg_holder = HashMap::new();
        // Folder is not required if files include in project.
        // Watch not supported for MessageHolder.
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
                println!("{}", e);
                Error::FileNotFound { path: folder }
            }).unwrap();
        let mut msg_holder = HashMap::new();

        for f in dir {
            let full_path = f.unwrap().path().to_str().unwrap().to_string();
            let holder = MessageHolder::new(full_path);
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

    // Maybe need getting new object are not referenced?
    pub fn get_by_locale(&self, locale: &str) -> Option<Data> {
        let holders = &self.holders;
        let holder = holders.get(locale)?;
        Some(Data::new(Arc::clone(&holder.messages)))
    }

    pub fn get_current_locale(&self) -> Option<Data> {
        let locale = get_current_locale_or_default();
        self.get_by_locale(&*locale)
    }

    pub fn get_by_locale_state(&self, locale: &str) -> Option<UnWatchData> {
        let holders = &self.holders;
        let holder = holders.get(locale)?;
        let read_state = holder.messages.read().unwrap();
        Some(UnWatchData::new(&read_state))
    }

    pub fn get_current_locale_state(&self) -> Option<UnWatchData> {
        let locale = get_current_locale_or_default();
        let state = self.get_by_locale_state(&*locale)?;
        Some(state)
    }

    pub fn add_provider(&mut self, locale: &str, provider: Box<dyn WatchProvider + 'static>) -> Result<(), Error> {
        let holder = self.holders.get(locale);
        let holder = holder.unwrap();
        holder.provider.replace(provider);
        holder.provider.borrow_mut().set_data(Arc::clone(&holder.messages))?;
        holder.provider.borrow_mut().watch()?;
        Ok(())
    }
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

    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<String> {
        return self.holder.get(key.as_ref()).map(|r| r.to_string());
    }

    pub fn get_or_default<S: AsRef<str>>(&self, key: S) -> String {
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

    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<String> {
        let state = self.holder.read().unwrap();
        return state.clone().get(key.as_ref()).map(|r| r.to_string());
    }

    pub fn get_or_default<S: AsRef<str>>(&self, key: S) -> String {
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

pub struct MessageHolder {
    messages: Arc<RwLock<HashMap<String, String>>>,
    locale: String,
    provider: RefCell<Box<dyn WatchProvider>>,
}

impl MessageHolder {
    pub fn new<S: Into<String>>(path: S) -> Result<MessageHolder, Error> {
        load_struct(path)
    }
}

impl WatchProvider for MessageHolder {
    fn watch(&mut self) -> Result<(), Error> {
        self.provider.borrow_mut().watch()
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.messages = data;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileStructure {
    kind: String,
    locale: String,
    description: Option<String>,
    provider: Option<Providers>,
    data: Option<HashMap<String, String>>,
}

fn load_struct_from_str(data: &str, path: Option<String>) -> Result<MessageHolder, Error> {
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
            Ok(MessageHolder {
                messages,
                locale,
                provider: RefCell::new(Box::new(StaticFileProvider {})),
            })
        }
        Some(p) => {
            match p {
                Providers::FileProvider => {
                    let provider = FileProvider::new(Arc::clone(&messages), path.clone());
                    Ok(MessageHolder {
                        messages,
                        locale,
                        provider: RefCell::new(Box::new(provider)),
                    })
                }
                Providers::StaticFileProvider => {
                    Ok(MessageHolder {
                        messages,
                        locale,
                        provider: RefCell::new(Box::new(StaticFileProvider {})),
                    })
                }
            }
        }
    };
}

fn load_struct<S: Into<String>>(path: S) -> Result<MessageHolder, Error> {
    let mut data = String::new();
    let path = path.into().trim_end().to_string();

    if !path.ends_with(".yaml") && !path.ends_with(".yml") {
        return Err(Error::NotSupportedFileExtension { path: path.clone() });
    }

    let mut file = File::open(&path)
        .map_err(|e| {
            log::error!("Error while open file {}. Additional information: {}", &path, e);
            Error::FileNotFound {
                path: path.clone()
            }
        })?;
    file.read_to_string(&mut data).unwrap();
    load_struct_from_str(&*data, Some(path))
}

fn get_locale_or_default(locale: &str) -> String {
    get_locale().unwrap_or(String::from(locale))
}

fn get_current_locale_or_default() -> String {
    get_locale_or_default("en-US")
}