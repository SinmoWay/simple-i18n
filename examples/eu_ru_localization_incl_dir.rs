use include_dir::{include_dir, Dir};
use simple_i18n::InternationalCore;

const PROJECT_DIR: Dir = include_dir!("examples/eu_ru");

fn main() {
    let core = InternationalCore::from(PROJECT_DIR);
    let eu = core.get_by_locale("EN");
    assert_eq!(true, eu.is_some());
    let ru = core.get_by_locale("RU");
    assert_eq!(true, ru.is_some());
    let eu_un = eu.unwrap();
    let ru_un = ru.unwrap();

    let eu_holder = eu_un.read().unwrap();
    let ru_holder = ru_un.read().unwrap();

    let eu_test = eu_holder.get("name").unwrap();
    assert_eq!("Test", &*eu_test);
    let ru_test = ru_holder.get("name").unwrap();
    assert_eq!("Тест", &*ru_test);
}