use crate::{GetData, InternationalCore};
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
    ($field:expr) => {
        {
            $crate::feature_macro::init($field)
        }
    }
}

/// Analogue `init_i18n!` but for feature `incl_dir`
#[cfg(feature = "incl_dir")]
#[macro_export]
macro_rules! init_i18n_static_dir {
    ($field:expr) => {
        {
            $crate::feature_macro::init_dir($field)
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
    ($field:expr, $field1:expr) => {
        {
            $crate::feature_macro::get_param($field, $field1)
        }
    };
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