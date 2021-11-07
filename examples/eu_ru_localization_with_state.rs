use sorrow_i18n::{GetData, InternationalCore};

fn main() {
    // Init core
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    let core = InternationalCore::new(manifest);

    // This method returns an unmodifiable reference to the value (internally).
    let eu = core.get_by_locale_state("EN");
    assert_eq!(true, eu.is_some());

    let ru = core.get_by_locale_state("RU");
    assert_eq!(true, ru.is_some());
    let eu_un = eu.unwrap();
    let ru_un = ru.unwrap();

    // We get the same key in two locales
    // Working with this holder, keys will never return other values.
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