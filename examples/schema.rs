use simple_json_parser::{parse, JSONKey, RootJSONValue};
use std::collections::{HashMap, HashSet};

fn main() {
    #[derive(Debug, Default)]
    struct Point {
        pub constraint: Constraint,
        pub description: String,
        pub title: String,
    }

    #[derive(Debug, Default)]
    struct Property {
        pub point: Point,
        pub required: bool,
    }

    type Properties = HashMap<String, Property>;

    // TODO root constraint...?
    #[derive(Debug, Default)]
    #[allow(unused)]
    enum Constraint {
        Object(Properties),
        String,
        Number {
            multiple: Option<f64>,
            // TODO could be better
            exclusive_minimum: Option<f64>,
            minimum: Option<f64>,
            exclusive_maximum: Option<f64>,
            maximum: Option<f64>,
        },
        Or(Box<Self>, Box<Self>),
        Array {
            constraint: Box<Point>,
            min_items: Option<usize>,
            unique_items: bool,
        },
        Boolean,
        #[default]
        Null,
    }

    impl<'a> TryFrom<&'a str> for Constraint {
        type Error = &'a str;

        fn try_from(value: &'a str) -> Result<Self, Self::Error> {
            match value {
                "object" => Ok(Constraint::Object(Default::default())),
                "string" => Ok(Constraint::String),
                "number" => Ok(Constraint::Number {
                    multiple: None,
                    exclusive_minimum: None,
                    minimum: None,
                    exclusive_maximum: None,
                    maximum: None,
                }),
                "integer" => Ok(Constraint::Number {
                    multiple: Some(1.),
                    exclusive_minimum: None,
                    minimum: None,
                    exclusive_maximum: None,
                    maximum: None,
                }),
                "array" => Ok(Constraint::Array {
                    constraint: Box::new(Default::default()),
                    min_items: None,
                    unique_items: false,
                }),
                name => {
                    // Constraint::Null
                    Err(name)
                } // Or(Box<Self>, Box<Self>),
                  // Array(Box<Constraint>),
                  // Boolean,
                  // #[default]
                  // Null,"
            }
        }
    }

    impl Constraint {
        pub fn get_property(&self, name: &str) -> Option<&Point> {
            if let Self::Object(properties) = self {
                properties.get(name).map(|property| &property.point)
            } else {
                None
            }
        }

        pub fn get_index(&self) -> Option<&Point> {
            if let Self::Array { ref constraint, .. } = self {
                Some(constraint)
            } else {
                None
            }
        }
    }

    #[derive(Debug, Default)]
    struct Scheme {
        pub id: String,
        pub value: Point,
    }

    let path = std::env::args()
        .nth(1)
        .unwrap_or("./private/corpus/schema/schema3.json".to_owned());
    let content = std::fs::read_to_string(path).unwrap();
    let mut top = Scheme::default();

    let _ = parse(&content, |keys, value| {
        if let &[JSONKey::Slice("$id")] = keys {
            if let RootJSONValue::String(id) = value {
                top.id = id.to_owned();
            } else {
                panic!();
            }
        } else {
            const ROOT_PROPERTIES: &[&str] = &[
                "type",
                "description",
                "title",
                "exclusiveMinimum",
                "minItems",
                "uniqueItems",
            ];

            enum Value<'a> {
                Property(&'a str, RootJSONValue<'a>),
                Required(&'a str),
            }

            let (root, value) = match keys {
                // [root @ .., JSONKey::Slice("items"), JSONKey::Slice(key)] if ROOT_PROPERTIES.contains(key) => {
                //     (root, Value::Property(*key, value))
                // },
                [root @ .., JSONKey::Slice(key)] if ROOT_PROPERTIES.contains(key) => {
                    (root, Value::Property(*key, value))
                }
                [root @ .., JSONKey::Slice("required"), JSONKey::Index(_)] => {
                    if let RootJSONValue::String(property_name) = value {
                        (root, Value::Required(property_name))
                    } else {
                        panic!();
                    }
                }
                keys => {
                    eprintln!("Unknown {keys:?}");
                    return;
                }
            };
            let mut item = &mut top.value;
            // TODO verify well formatter
            for pair in root.chunks(2) {
                if let [JSONKey::Slice("properties"), JSONKey::Slice(id)] = pair {
                    if let Constraint::Object(ref mut properties) = item.constraint {
                        let entry = properties.entry(id.to_string()).or_default();
                        item = &mut entry.point;
                    } else {
                        eprintln!("not an object!!");
                    }
                } else if let [JSONKey::Slice("items")] = pair {
                    if let Constraint::Array {
                        ref mut constraint, ..
                    } = item.constraint
                    {
                        item = constraint;
                    } else {
                        eprintln!("not an array!!");
                    }
                } else {
                    // else if let [JSONKey::Slice(id),  {
                    //     if let Constraint::Object(ref mut properties) = item.constraint {
                    //         let entry = properties.entry(id.to_string()).or_default();
                    //         if let Constraint::Array(ref mut inner) = entry.point.constraint {
                    //             item = inner;
                    //         } else {
                    //             eprintln!("not an array");
                    //         }
                    //     } else {
                    //         eprintln!("not an object!!");
                    //     }
                    // }
                    eprintln!("TODO pair is {pair:?}")
                }
            }

            match value {
                Value::Property(key, value) => {
                    match key {
                        "type" => {
                            if let RootJSONValue::String(constraint) = value {
                                item.constraint = match Constraint::try_from(constraint) {
                                    Ok(constraint) => constraint,
                                    Err(name) => {
                                        eprintln!("TODO type is {name:?}");
                                        Default::default()
                                    }
                                };
                                // eprintln!("{:?}", &item.constraint);
                            } else {
                                panic!();
                            }
                        }
                        "description" => {
                            if let RootJSONValue::String(description) = value {
                                item.description = description.to_owned();
                            } else {
                                panic!();
                            }
                        }
                        "title" => {
                            if let RootJSONValue::String(title) = value {
                                item.title = title.to_owned();
                            } else {
                                panic!();
                            }
                        }
                        "exclusiveMinimum" => {
                            if let RootJSONValue::Number(value) = value {
                                if let Constraint::Number {
                                    ref mut exclusive_minimum,
                                    ..
                                } = item.constraint
                                {
                                    *exclusive_minimum = Some(value.parse().unwrap());
                                }
                            } else {
                                panic!();
                            }
                        }
                        "minItems" => {
                            if let RootJSONValue::Number(value) = value {
                                if let Constraint::Array {
                                    ref mut min_items, ..
                                } = item.constraint
                                {
                                    *min_items = Some(value.parse().unwrap());
                                }
                            } else {
                                panic!();
                            }
                        }
                        "uniqueItems" => {
                            if let RootJSONValue::Boolean(value) = value {
                                if let Constraint::Array {
                                    ref mut unique_items,
                                    ..
                                } = item.constraint
                                {
                                    *unique_items = value;
                                }
                            } else {
                                panic!();
                            }
                        }
                        key => {
                            eprintln!("TODO property {key}")
                        }
                    }
                }
                Value::Required(property_name) => {
                    if let Constraint::Object(ref mut properties) = item.constraint {
                        // TODO entry?
                        properties
                            .entry(property_name.to_string())
                            .or_default()
                            .required = true
                    } else {
                        eprintln!("not an object!!");
                    }
                }
            }
        }
    });

    // dbg!(&top);

    let path = std::env::args()
        .nth(2)
        .unwrap_or("./private/corpus/schema/schema3_case.json".to_owned());
    let content = std::fs::read_to_string(path).unwrap();

    let mut last_chain: Vec<JSONKey<'_>> = Vec::new();
    let mut unvisited_properties: Vec<HashSet<&'_ str>> = vec![HashSet::new(); 10];
    if let Constraint::Object(ref properties) = top.value.constraint {
        unvisited_properties[0] = properties
            .iter()
            .filter_map(|(name, property)| property.required.then_some(name.as_str()))
            .collect();
    }

    let _ = parse(&content, |keys, value| {
        if let [JSONKey::Slice("$schema")] = keys {
            return;
        }
        // eprintln!("unvisited_properties = {unvisited_properties:?}");
        let mut item = &top.value;
        // let mut last = &top.value;
        for (idx, key) in keys.iter().enumerate() {
            match key {
                JSONKey::Slice(key) => {
                    if let Constraint::Object(ref _properties) = item.constraint {
                        if idx == keys.len() - 1 {
                            let missing = unvisited_properties[idx].remove(key);
                            if !missing {
                                eprintln!("Duplicate / or excess {key:?}");
                            }
                        } else {
                            eprintln!("TODO update unvisited properties");
                        }
                    }
                    if let Some(p) = item.constraint.get_property(key) {
                        item = p
                    } else {
                        eprintln!("No constraint for {key:?}");
                        return;
                    }
                }
                JSONKey::Index(_) => {
                    // TODO need idx sometiems
                    if let Some(p) = item.constraint.get_index() {
                        item = p
                    } else {
                        eprintln!("No constraint for array");
                        return;
                    }
                }
            }
            // last = item;
        }
        match (&item.constraint, value) {
            (Constraint::Number { multiple, .. }, RootJSONValue::Number(number)) => {
                let number: f64 = number.parse().unwrap();
                if let Some(multiple) = multiple {
                    if number % multiple != 0. {
                        eprintln!("Bad multiple");
                        return;
                    }
                }
                eprintln!("Okay {number:?}")
            }
            (Constraint::String, RootJSONValue::String(string)) => {
                eprintln!("Okay {string:?}")
            }
            (Constraint::Boolean, RootJSONValue::Boolean(value)) => {
                eprintln!("Okay {value:?}")
            }
            (Constraint::Null, RootJSONValue::Null) => {
                eprintln!("Okay null")
            }
            (constraint, value) => {
                eprintln!("Value {value:?} does not meet constraint {constraint:?}");
                eprintln!("{keys:?}, {value:?}, {item:?}");
            }
        }
        // TODO use similar method to formatting
        last_chain = keys.to_owned();
    });

    eprintln!("unvisited_properties={unvisited_properties:?}")
}
