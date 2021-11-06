use include_dir::{include_dir, Dir};
use sorrow_i18n::{init_i18n_static_dir, i18n};

const PROJECT_DIR: Dir = include_dir!("resources/en_ru");

fn main() {
    init_i18n_static_dir!(PROJECT_DIR);
    let test = i18n!("RU", "data.name");
    assert_eq!("Тест", &*test);
    let not_found_data = i18n!("RU", "data.not_found_me");
    assert_eq!("data.not_found_me", &*not_found_data);
    assert_eq!("Test", i18n!("EN", "data.name"));
}