# Simple i18n

[![ci](https://github.com/SinmoWay/simple-i18n/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/SinmoWay/simple-i18n/actions/workflows/ci.yml)  
Simple implementation to load locale.

## Dependency

### Base

Basic usage assumes that you are not using any features in the project.

```toml
[dependencies]
sorrow-i18n = "0.1.0"
```

### Features

#### incl_dir

A simple feature for loading static files into a project.

Usage:

```toml
[dependencies]
sorrow-i18n = { version = "0.1.0", features = ["incl_dir"] }
```

## Docs

TODO

## Usages

### Base usage with static files

Firstly add dependency `sorrow-i18n` in your project. A typical view of the structure of a file is `FileStructure`. Data
scheme - `scheme/locale-scheme.json`. And so, we will create the actual minimal file for work:

```yaml
kind: I18N
locale: EN
description: test en
data:
  name: "Test"
```

And the second localization:

```yaml
kind: I18N
locale: RU
description: test ru
data:
  name: "Тест"
```

Now it's time for the codebase, the files we created earlier will be placed, for example, in `locale/`.

```
use sorrow_i18n::{GetData, InternationalCore};
let core = InternationalCore::new("locale/");
```

Having created the core, we can get our localizations and work with them.

```
let eu = core.get_by_locale("EN")?;
let ru = core.get_by_locale("RU")?;
```

OR

```
let eu = core.get_by_locale("EN")?;
let ru = core.get_by_locale("RU")?;
```

What are the differences between these methods? First, when the `get_by_locale` method is called, a reference to the
mutable data is returned, and in the case of `get_by_locale`, you just get a reference to the data (in both cases, you
are working with a wrapper) that cannot change from outside. Plus the first approach, if your localization can change
during the execution of the program, then you will receive up-to-date data, but there are additional costs for blocking.
Plus the second approach, if your files are static, loaded into the project, and they cannot be changed, in this case,
you just work with `HashMap`.  
But how can we track or choose a file tracking strategy? And how are we going to get those updates? The provider will
help us with this! By default, we provide you with several of these, namely:

* `StaticFileProvider` - static file. It is not being watched. Default option if the `provider` is not specified in the
  file structure
* `FileProvider` - dynamically watcher for file. If the provider is not specified, the default is `StaticFileProvider`.
  The question remains, how to choose this provider? Simple enough, here's an example where the provider is explicitly
  specified:

```yaml
kind: I18N
locale: RU
description: test ru
provider: FileProvider
data:
  name: "Тест"
```

OR

```yaml
kind: I18N
locale: RU
description: test ru
provider: StaticFileProvider
data:
  name: "Тест"
```

You can read more about providers below.  
Finally, we got our locales, it remains to get what we want! Namely: `data.name`.

```
 assert_eq!("Test", eu.get("data.name")?);
 assert_eq!("Тест", ru.get("data.name")?);
```

As you can see, everything is quite simple, there is also a similar method for getting the value by default (this will
be the same key that you requested)

```
 assert_eq!("Test", eu.get_or_default("data.name"));
 assert_eq!("Тест", ru.get_or_default("data.name"));
 assert_eq!("keykey", eu.get_or_default("keykey"));
```

You can see more examples in `examples/*`

# Providers

As we said earlier, the provider is responsible for the data update strategy. Its main method is watch.

```
pub trait WatchProvider {
    /// The main observer method that is called to observe the state.
    fn watch(&mut self) -> Result<(), Error>;

    /// Setter for data reference.
    fn set_data(&mut self, data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error>;
}
```
## StaticFileProvider

It is he who observes the change in data. In the case of a static observer, we have an empty method because we don't
need to observe.

```
impl WatchProvider for StaticFileProvider {
    fn watch(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn set_data(&mut self, _data: Arc<RwLock<HashMap<String, String>>>) -> Result<(), Error> {
        Ok(())
    }
}
```

## FileProvider
But with FileProvider, everything is not much more complicated. The `notify` library is used to constantly monitor the state of the file. The only event tracked is the file change.
```
let event = result.map_err(|e| Error::WatchError { message: e.to_string() }).unwrap();
if event.kind.is_modify() {
    ...
}
```
Every time we change the file, we get a lock on our data, but first we load the updated file itself (to validate the structure). Actually poisoning the blockage in this way is very, very difficult.
```
// Validation file
let structure = load_struct(&path.clone()).unwrap();

// Lock data and clear
let mut w_holder = holder.write().unwrap();
w_holder.clear();

// Clone internal state.
let l_holder = structure.messages.write().unwrap().clone();
w_holder.extend(l_holder);
```
## Custom provider
There are situations when it is necessary, for example, to load project locales first, and later maintain a connection to a database or some other data source, to constantly update the data itself. For this we can create our own data provider! The simplest example and illustrative example is in `examples/custom_provider.rs`  
Well, now, point by point, to begin with, let's create a simple structure that will monitor our data.
```
pub struct CustomProvider {
    data: Arc<RwLock<HashMap<String, String>>>,
}
```
And we will implement our provider for it:
```
impl WatchProvider for CustomProvider {
    fn watch(&mut self) -> Result<(), sorrow_i18n::Error> {
        println!("Accepted custom provider");
        let data = self.data.write();
        let mut un = data.unwrap();
        // Print all current data
        un.iter().for_each(|kv| {
            println!("Key: {}, Value: {}", kv.0, kv.1);
        });
        // Add new key
        un.insert("Hello".to_string(), "World".to_string());
        // Print all data, current data has been contains key "Hello"
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
```
There is a minimum left, to add our data provider to any locale.
```
    let mut core = InternationalCore::new("locale/");
    core.add_provider("EN", Box::new(CustomProvider::new()))?;
```
If such a locale exists, the following actions will be performed:
* `holder` -> getting current data 
* `provider` -> `set_data(current_data_in_holder)`
* `provider` -> `watch()`