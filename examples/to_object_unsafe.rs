//! Uses iterations rather than recursion to build a object map. Unfortunately at the cost of using `unsafe`

use std::collections::HashMap;

use simple_json_parser::{parse, JSONKey, RootJSONValue};

fn main() {
    let content = r#"{
        "name": "ezno",
        "version": "0.0.14",
        "description": "A JavaScript compiler and TypeScript checker written in Rust with a focus on static analysis and runtime performance",
        "license": "MIT",
        "repository": "https://github.com/kaleidawave/ezno",
        "main": "./dist/index.mjs",
        "module": "./dist/index.mjs",
        "type": "module",
        "exports": {
            ".": {
                "import": "./dist/index.mjs"
            },
            "./initialised": {
                "import": "./dist/initialised.mjs"
            }
        },
        "scripts": {
            "clean": "rmdir dist && rmdir build",
            "build": "cargo build --lib --target wasm32-unknown-unknown && npm run bind && npm run build-js",
            "build-release": "cargo build --lib --release --target wasm32-unknown-unknown && npm run bind-release && npm run build-js",
            "bind": "wasm-bindgen --out-dir build --target web ../../target/wasm32-unknown-unknown/debug/ezno_lib.wasm",
            "bind-release": "wasm-bindgen --out-dir build --target web ../../target/wasm32-unknown-unknown/release/ezno_lib.wasm",
            "build-js": "unbuild && cp ./build/ezno_lib_bg.wasm dist/shared && cp src/cli_node.cjs dist/cli.cjs",
            "test": "npm run build && npm run run-tests",
            "run-tests": "node test.mjs && deno run -A test.mjs"
        },
        "keywords": [
            "typescript",
            "checker",
            "type-checker",
            "compiler"
        ],
        "files": [
            "dist"
        ],
        "bin": {
            "ezno": "./dist/cli.mjs"
        },
        "author": {
            "name": "Ben",
            "email": "kaleidawave@gmail.com",
            "url": "https://kaleidawave.github.io/"
        },
        "funding": {
            "type": "individual",
            /*
                multiline comment
             */
            "url": "https://github.com/sponsors/kaleidawave"
        },
        "build": {
            "failOnWarn": false,
            "entries": [
                {
                    "builder": "rollup",
                    "input": "./src/index"
                },
                {
                    "builder": "rollup",
                    "input": "./src/initialised"
                },
                {
                    "builder": "rollup",
                    "input": "./src/cli"
                }
            ],
            // some comment
            "rollup": {
                "commonjs": true,
                "esbuild": {
                    "target": "esnext"
                }
            }
        },
        "devDependencies": {
            "unbuild": "^1.1.2"
        }
    }"#;

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

    let result = parse(content, |keys, value| {
        let [path @ .., end] = keys else {
            unreachable!("empty key change")
        };
        let pointer = &mut root;

        let mut to_add_to: *mut Object = pointer;

        for key in path {
            let name = match key {
                JSONKey::Slice(s) => s.to_string(),
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
            JSONKey::Slice(s) => s.to_string(),
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

    match result {
        Ok(()) => {
            eprintln!("Object:\n{:#?}", root);
        }
        Err((idx, err)) => eprintln!("{err:?} @ {idx}"),
    }
}
