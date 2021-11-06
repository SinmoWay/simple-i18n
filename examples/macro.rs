use sorrow_i18n::{i18n, init_i18n};

fn main() {
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    init_i18n!(manifest);
    let test = i18n!("RU", "data.name");
    println!("test: {}", &*test);
    assert_eq!("Тест", &*test);
    let not_found_data = i18n!("RU", "data.not_found_me");
    println!("data not found: {}", &*not_found_data);
    assert_eq!("data.not_found_me", &*not_found_data);
}