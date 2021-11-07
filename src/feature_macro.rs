use crate::{GetData, InternationalCore, WatchProvider};
use std::sync::{RwLock};
use once_cell::sync::Lazy;
#[cfg(feature = "incl_dir")]
use include_dir::Dir;

static I18N_CORE: Lazy<RwLock<Vec<InternationalCore>>> = Lazy::new(|| { RwLock::new(vec![]) });

/// We statically initialize our core. In case of reinitialization, we panic.
///
/// # Arguments
///
/// * Arg - locale folder.
///
/// # Examples
///
/// ```
/// init_i18n!("locale/");
/// ```
/// Run function `sorrow_i18n::feature_macro::init`
#[macro_export]
macro_rules! init_i18n {
    ($path:expr) => {
        {
            $crate::feature_macro::init($path)
        }
    }
}

/// Analogue `init_i18n!` but for feature `incl_dir`
#[cfg(feature = "incl_dir")]
#[macro_export]
macro_rules! init_i18n_static_dir {
    ($dir:expr) => {
        {
            $crate::feature_macro::init_dir($dir)
        }
    }
}

/// Get a value from the store using the locale and key.
///
/// # Arguments
/// * First argument - locale
/// * Second argument - key
///
/// # Examples
/// ```
///  // First init core
///  init_i18n!("locale/");
///  // Getting data
///  let test = i18n!("RU", "data.name");
///  assert_eq!("Тест", &*test);
///  // / If the key is not found or the locale is not found, return the passed key.
///  let not_found_data = i18n!("RU", "data.not_found_me");
///  assert_eq!("data.not_found_me", &*not_found_data);
/// ```
///
/// Run function `crate::feature_macro::get_param`
#[macro_export]
macro_rules! i18n {
    ($locale:expr, $key:expr) => {
        {
            $crate::feature_macro::get_param($locale, $key)
        }
    };
}

/// Setting custom provider by holder.
///
/// # Arguments
/// * first argument - locale
/// * second argument - provider
///
/// # Examples
/// ```rust
///     use sorrow_i18n::{init_i18n, set_i18n_provider};
///     init_i18n!("locale/");
///     let provider = Box::new(CustomProvider::new());
///     set_i18n_provider!("EE", provider);
/// ```
/// [Full example](https://github.com/SinmoWay/simple-i18n/blob/main/examples/macro_with_custom_provider.rs)
#[macro_export]
macro_rules! set_i18n_provider {
    ($locale:expr, $provider:expr) => {
        {
            $crate::feature_macro::set_provider($locale, $provider)
        }
    }
}

/// We statically initialize our core. In case of reinitialization, we panic.
pub fn init<S: AsRef<str>>(_path: S) {
    check_empty_core();
    let mut core_holder = I18N_CORE.write().unwrap();
    let core = InternationalCore::new(_path.as_ref().to_string());
    core_holder.insert(0, core);
}

#[cfg(feature = "incl_dir")]
/// Analogue `init` only for feature `incl_dir`
pub fn init_dir(dir: Dir) {
    check_empty_core();
    let mut core_holder = I18N_CORE.write().unwrap();
    let core = InternationalCore::from(dir);
    core_holder.insert(0, core);
}

fn check_empty_core() {
    let mut err = false;

    {
        let core = I18N_CORE.write().unwrap();

        if !core.is_empty() {
            log::error!("Double init I18N core.");
            err = true;
        }
    }


    if err {
        panic!("Error while init i18n core. Core has been init.");
    }
}

/// Get a value from the store using the locale and key.
pub fn get_param(locale: &str, key: &str) -> String {
    let guard = I18N_CORE.read().unwrap();

    match guard.get(0) {
        None => {
            key.to_string()
        }
        Some(c) => {
            match c.get_by_locale(locale) {
                None => {
                    key.to_string()
                }
                Some(h) => {
                    h.get_or_default(key)
                }
            }
        }
    }
}

/// Add custom provider for locale holder
pub fn set_provider(locale: &str, provider: Box<dyn WatchProvider + 'static + Sync + Send>) {
    let mut guard = I18N_CORE.write().unwrap();

    match guard.get_mut(0) {
        None => {
            panic!("The i18n core has not been created. Call the init_i18n! or init_i18n_static_dir! macro.");
        }
        Some(core) => {
            match core.add_provider(&locale, provider) {
                Ok(_) => {
                    log::debug!("Provider has been accepted for locale: {}", &locale)
                }
                Err(e) => {
                    log::error!("Error while add provider for locale {}", &locale);
                    panic!("{:?}", e);
                }
            }
        }
    }
}