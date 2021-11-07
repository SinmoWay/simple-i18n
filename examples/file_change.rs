use std::thread::sleep;
use std::time::Duration;
use sorrow_i18n::{GetData, InternationalCore};

// A visual demonstration of changing the file and changing values in the holders.
fn main() {
    // Init core
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    let core = InternationalCore::new(manifest.clone());

    // This method returns a mutable reference to the value (internally).
    let ru_locale = core.get_by_locale("RU").unwrap();
    let mut name = ru_locale.get_or_default("data.name");
    assert_eq!("Тест", name);

    // Open file and replace data.name key with value `Тест`
    let ru_path = format!("{}/I18N_RU.yaml", &manifest);
    let mut data = std::fs::read_to_string(&ru_path).unwrap();
    data = data.replace("Тест", "Хей! Как ты?");
    std::fs::write(&ru_path, data.as_bytes()).unwrap();

    // Await change
    sleep(Duration::from_millis(100));

    // We get the same key, but we get the already changed value.
    name = ru_locale.get("data.name").unwrap();
    assert_eq!("Хей! Как ты?", name);

    // Overwrite changes
    data = data.replace("Хей! Как ты?", "Тест");
    std::fs::write(&ru_path, data.as_bytes()).unwrap();
    sleep(Duration::from_millis(100));

    // We check if everything is correct.
    name = ru_locale.get_or_default("data.name");
    assert_eq!("Тест", name);
}