use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use sorrow_i18n::{WatchProvider, init_i18n, set_i18n_provider, i18n, Error};

fn main() {
    let manifest = format!("{}{}", env!("CARGO_MANIFEST_DIR"), "/resources/en_ru");
    init_i18n!(manifest);
    let provider = Box::new(CustomProvider::new());
    set_i18n_provider!("EE", provider);
    let test = i18n!("EE", "data.name");
    println!("test: {}", &*test);
    assert_eq!("Helly belly", &*test);
    let not_found_data = i18n!("EE", "data.not_found_me");
    println!("data not found: {}", &*not_found_data);
    assert_eq!("data.not_found_me", &*not_found_data);
    let hello = i18n!("EE", "Hello");
    println!("Hello key: {}", &*hello);
    assert_eq!("World", &*hello);
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
        println!("Add Hello key and value World");
        un.insert("Hello".to_string(), "World".to_string());
        Ok(())
    }

    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        self.data = data;
        println!("Data has been set");
        Ok(())
    }
}