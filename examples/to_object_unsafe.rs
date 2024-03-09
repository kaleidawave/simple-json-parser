//! Uses iterations rather than recursion to build a object map. Unfortunately at the cost of using `unsafe`

use std::collections::HashMap;

use simple_json_parser::{parse, JSONKey, RootJSONValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Expected first argument")?;
    let content = std::fs::read_to_string(path)?;

    pub type Object = HashMap<String, Value>;

    #[derive(Debug)]
    pub enum Value {
        Object(Object),
        String(String),
        Number(String),
        Boolean(bool),
        Null,
    }

    impl Value {
        pub fn new_empty_object() -> Self {
            Self::Object(HashMap::new())
        }
    }

    let mut root = Object::new();

    let _res = parse(&content, |keys, value| {
        let [path @ .., end] = keys else {
            unreachable!("empty key change")
        };
        let pointer = &mut root;

        let mut to_add_to: *mut Object = pointer;

        for key in path {
            let name = match key {
                JSONKey::Slice(s) => (*s).to_string(),
                JSONKey::Index(i) => i.to_string(),
            };
            if let Some(Value::Object(ref mut obj)) =
                unsafe { (to_add_to.as_mut().unwrap()).get_mut(&name) }
            {
                to_add_to = obj;
            } else {
                let value = unsafe {
                    (to_add_to.as_mut().unwrap())
                        .entry(name)
                        .or_insert_with(Value::new_empty_object)
                };
                if let Value::Object(ref mut obj) = value {
                    to_add_to = obj;
                }
            }
        }
        let name = match end {
            JSONKey::Slice(s) => (*s).to_string(),
            JSONKey::Index(i) => i.to_string(),
        };
        let value = match value {
            RootJSONValue::String(s) => Value::String(s.to_string()),
            RootJSONValue::Number(n) => Value::Number(n.to_string()),
            RootJSONValue::True => Value::Boolean(true),
            RootJSONValue::False => Value::Boolean(false),
            RootJSONValue::Null => Value::Null,
        };
        unsafe {
            (to_add_to.as_mut().unwrap()).insert(name, value);
        }
    });

    eprintln!("Object:\n{root:#?}");
    Ok(())
}
