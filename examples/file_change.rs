use std::thread::sleep;
use std::time::Duration;
use simple_i18n::{GetData, InternationalCore};

fn main() {
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    let core = InternationalCore::new(manifest.clone());
    let ru_locale = core.get_by_locale("RU").unwrap();
    let mut name = ru_locale.get_or_default("data.name");
    assert_eq!("Тест", name);
    let ru_path = format!("{}/I18N_RU.yaml", &manifest);
    let mut data = std::fs::read_to_string(&ru_path).unwrap();
    data = data.replace("Тест", "Хей! Как ты?");
    std::fs::write(&ru_path, data.as_bytes()).unwrap();
    sleep(Duration::from_millis(100));
    name = ru_locale.get("data.name").unwrap();
    assert_eq!("Хей! Как ты?", name);
    data = data.replace("Хей! Как ты?", "Тест");
    std::fs::write(&ru_path, data.as_bytes()).unwrap();
    sleep(Duration::from_millis(10));
    name = ru_locale.get_or_default("data.name");
    assert_eq!("Тест", name);
}