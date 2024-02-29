mod bindings;

use crate::bindings::wasi::keyvalue::crud;
use crate::bindings::wasi::keyvalue::types;
use crate::bindings::Guest;

struct Component;

impl Guest for Component {
    fn doing(input: String) -> String {
        println!("Started doing");
        let store = types::Store::get("id").expect("Could not get store");
        let handle = crud::Crud::open(&store);
        crud::Crud::set(&handle, &input, "foo".as_bytes()).expect("Could not set value");
        let val = crud::Crud::get(&handle, &input).expect("Could not get value");
        format!(
            "Value for key {} is: {}",
            input,
            std::str::from_utf8(&val.unwrap()).expect("Could not convert to string")
        )
    }
}
