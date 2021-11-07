use include_dir::{include_dir, Dir};
use sorrow_i18n::{GetData, InternationalCore};

// Init static dir
const PROJECT_DIR: Dir = include_dir!("resources/en_ru");

fn main() {
    // Init core
    let core = InternationalCore::from(PROJECT_DIR);

    // Getting EN locale
    // This method returns a mutable reference to the value (internally).
    let eu = core.get_by_locale("EN");
    assert_eq!(true, eu.is_some());

    // Getting RU locale
    let ru = core.get_by_locale("RU");
    assert_eq!(true, ru.is_some());
    let eu_un = eu.unwrap();
    let ru_un = ru.unwrap();

    // We get the same key in two locales
    let eu_name = eu_un.get("data.name");
    let ru_name = ru_un.get("data.name");

    assert_eq!(true, eu_name.is_some());
    assert_eq!("Test", eu_name.unwrap());

    assert_eq!(true, ru_name.is_some());
    assert_eq!("Тест", ru_name.unwrap());

    // We return the key, because it does not exist.
    assert_eq!("data.modify", eu_un.get_or_default("data.modify"));

    // Key vector
    let keys = ru_un.keys();
    assert_eq!(1usize, keys.len());
    assert_eq!("data.name", keys.get(0).unwrap());
}