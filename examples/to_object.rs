use std::collections::HashMap;

use simple_json_parser::{parse, JSONKey, RootJSONValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Expected first argument")?;
    let content = std::fs::read_to_string(path)?;

    pub type Object = HashMap<String, Value>;
    pub type Array = Vec<Value>;

    #[derive(Debug)]
    #[allow(dead_code)]
    pub enum Value {
        Object(Object),
        Array(Array),
        String(String),
        Number(f64),
        Boolean(bool),
        Null,
    }

    impl From<RootJSONValue<'_>> for Value {
        fn from(value: RootJSONValue<'_>) -> Self {
            match value {
                RootJSONValue::String(s) => Value::String(s.value().to_string()),
                RootJSONValue::Number(n) => Value::Number(n.value_unwrap()),
                RootJSONValue::Boolean(v) => Value::Boolean(v),
                RootJSONValue::Null => Value::Null,
                RootJSONValue::EmptyObject => Value::new_empty_object(),
                RootJSONValue::EmptyArray => Value::new_empty_array(),
                RootJSONValue::Comment(_) | RootJSONValue::Empty => {
                    eprintln!("Option should have been turned off");
                    Value::Null
                }
            }
        }
    }

    impl Value {
        pub fn new_empty_object() -> Self {
            Self::Object(HashMap::new())
        }

        pub fn new_empty_array() -> Self {
            Self::Array(Vec::new())
        }

        pub fn set<'a>(&'a mut self, keys: &'a [JSONKey<'a>], value: RootJSONValue<'a>) {
            if let [last] = keys {
                let value = Value::from(value);
                match last {
                    JSONKey::Slice(s) => {
                        let Value::Object(ref mut obj) = self else {
                            unreachable!("parsing broke");
                        };
                        let name = (*s).to_string();
                        let existing = obj.insert(name, value);
                        debug_assert!(existing.is_none());
                    }
                    JSONKey::Index(_i) => {
                        let Value::Array(ref mut array) = self else {
                            unreachable!("parsing broke");
                        };
                        array.push(value);
                    }
                };
            } else if let [first, others @ ..] = keys {
                match first {
                    JSONKey::Slice(s) => {
                        let Value::Object(ref mut obj) = self else {
                            unreachable!("parsing broke");
                        };
                        let name = (*s).to_string();
                        let entry = obj.entry(name);
                        let object = if let JSONKey::Index(_) = others.first().unwrap() {
                            entry.or_insert_with(Value::new_empty_array)
                        } else {
                            entry.or_insert_with(Value::new_empty_object)
                        };
                        object.set(others, value);
                    }
                    JSONKey::Index(_i) => {
                        let Value::Array(ref mut array) = self else {
                            unreachable!("parsing broke");
                        };
                        let object = if let JSONKey::Index(_) = others.first().unwrap() {
                            if let Some(last) = array.last_mut() {
                                last
                            } else {
                                array.push(Value::new_empty_array());
                                array.last_mut().unwrap()
                            }
                        } else {
                            if let Some(last) = array.last_mut() {
                                last
                            } else {
                                array.push(Value::new_empty_object());
                                array.last_mut().unwrap()
                            }
                        };
                        object.set(others, value);
                    }
                };
            } else {
                unreachable!("empty keys")
            }
        }
    }

    // TODO if starts with '[' & comments
    let mut root = Value::new_empty_object();

    parse(&content, |keys, value| root.set(keys, value))?;

    eprintln!("Parsed: {root:#?}");
    Ok(())
}
