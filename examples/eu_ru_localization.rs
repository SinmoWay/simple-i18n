use sorrow_i18n::{GetData, InternationalCore};

fn main() {
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    let core = InternationalCore::new(manifest);
    let eu = core.get_by_locale("EN");
    assert_eq!(true, eu.is_some());
    let ru = core.get_by_locale("RU");
    assert_eq!(true, ru.is_some());
    let eu_un = eu.unwrap();
    let ru_un = ru.unwrap();

    let eu_name = eu_un.get("data.name");
    let ru_name = ru_un.get("data.name");

    assert_eq!(true, eu_name.is_some());
    assert_eq!("Test", eu_name.unwrap());

    assert_eq!(true, ru_name.is_some());
    assert_eq!("Тест", ru_name.unwrap());

    // Return Key as this.
    assert_eq!("data.modify", eu_un.get_or_default("data.modify"));

    let representation = eu_un.get("data.representation.yes");
    assert_eq!(true, representation.is_some());
    assert_eq!("No!", representation.unwrap());
    let currency = eu_un.get("data.currency.a");
    assert_eq!(true, currency.is_some());
    assert_eq!("No definition", currency.unwrap());
    let data_block = eu_un.get("data.data.block");
    assert_eq!(true, data_block.is_some());
    assert_eq!("test1", data_block.unwrap());
    let data_wok = eu_un.get("data.data.wok");
    assert_eq!(true, data_wok.is_some());
    assert_eq!("test2", data_wok.unwrap());

}