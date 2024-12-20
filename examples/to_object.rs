use std::collections::HashMap;

use simple_json_parser::{parse, JSONKey, RootJSONValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Expected first argument")?;
    let content = std::fs::read_to_string(path)?;

    pub type Object = HashMap<String, Value>;

    #[derive(Debug)]
    #[allow(dead_code)]
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

        pub fn set<'a>(&'a mut self, keys: &'a [JSONKey<'a>], value: RootJSONValue<'a>) {
            if let Value::Object(ref mut obj) = self {
                if let [last] = keys {
                    let name = match last {
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
                    let existing = obj.insert(name, value);
                    debug_assert!(existing.is_none());
                } else if let [first, others @ ..] = keys {
                    let name = match first {
                        JSONKey::Slice(s) => (*s).to_string(),
                        JSONKey::Index(i) => i.to_string(),
                    };
                    obj.entry(name)
                        .or_insert_with(Value::new_empty_object)
                        .set(others, value);
                } else {
                    unreachable!("empty keys")
                }
            } else {
                unreachable!()
            }
        }
    }

    let mut root = Value::new_empty_object();

    parse(&content, |keys, value| root.set(keys, value))?;

    eprintln!("Parsed: {root:#?}");
    Ok(())
}
