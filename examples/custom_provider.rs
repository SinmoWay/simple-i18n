use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use sorrow_i18n::{Error, GetData, InternationalCore, WatchProvider};

fn main() {
    // Init core
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    let mut core = InternationalCore::new(manifest);

    // Add custom provider for EE locale
    core.add_provider("EE", Box::new(CustomProvider::new())).unwrap();

    // We get data by EE locale
    // This method returns a mutable reference to the value (internally).
    let ee_opt = core.get_by_locale("EE");
    assert_eq!(true, ee_opt.is_some());
    let ee = ee_opt.unwrap();

    // Key exists in file
    let val = ee.get_or_default("data.name");
    assert_eq!("Helly belly", val);

    // The key does not exist in the file, but we add it through the provider.
    let hello = ee.get_or_default("Hello");
    assert_eq!("World", hello);
}

pub struct CustomProvider {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl CustomProvider {
    pub fn new() -> Self {
        CustomProvider {
            data: Arc::new(RwLock::new(HashMap::new()))
        }
    }
}

impl WatchProvider for CustomProvider {
    fn watch(&mut self) -> Result<(), sorrow_i18n::Error> {
        println!("Accepted custom provider");
        let data = self.data.write();
        let mut un = data.unwrap();
        println!("Current I18N_EE.yml data holder...");
        un.iter().for_each(|kv| {
            println!("Key: {}, Value: {}", kv.0, kv.1);
        });
        println!("Add Hello key and value World");
        un.insert("Hello".to_string(), "World".to_string());
        un.iter().for_each(|kv| {
            println!("Key: {}, Value: {}", kv.0, kv.1);
        });
        Ok(())
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.data = data;
        println!("Data has been set");
        Ok(())
    }
}