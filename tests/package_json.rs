use simple_json_parser::{key_chain_equals, parse_advanced, JSONKey, ParseOptions, RootJSONValue};

#[test]
fn parse_package_json() {
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
        "some_number": 4,
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

    let mut found = 0;
    let options = ParseOptions {
        allow_comments: true,
        ..Default::default()
    };

    let result = parse_advanced::<()>(content, &options, |keys, value| {
        if let &[JSONKey::Slice("name")] = keys {
            found += 1;
            assert_eq!(value, RootJSONValue::String("ezno"));
        } else if let &[JSONKey::Slice("author"), JSONKey::Slice("name")] = keys {
            found += 1;
            assert_eq!(value, RootJSONValue::String("Ben"));
        } else if let &[JSONKey::Slice("keywords"), JSONKey::Index(3)] = keys {
            found += 1;
            assert_eq!(value, RootJSONValue::String("compiler"));
        } else if key_chain_equals(
            keys,
            &[JSONKey::Slice("build"), JSONKey::Slice("failOnWarn")],
        ) {
            found += 1;
            assert_eq!(value, RootJSONValue::Boolean(false));
        }
        None
    });

    assert!(result.is_ok());
    assert_eq!(found, 4);
}
