use include_dir::{include_dir, Dir};
use simple_i18n::{GetData, InternationalCore};

const PROJECT_DIR: Dir = include_dir!("resources/en_ru");

fn main() {
    let core = InternationalCore::from(PROJECT_DIR);
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
    assert_eq!("data.modify", eu_un.get_or_default("data.modify"))
}