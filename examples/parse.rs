use simple_json_parser::{parse_advanced, JSONParseError, ParseOptions};

fn main() {
    let base = r#"{
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

    let content = if let Some(path) = std::env::args().nth(1) {
        std::borrow::Cow::Owned(std::fs::read_to_string(path).unwrap())
    } else {
        std::borrow::Cow::Borrowed(base)
    };

    let partial_syntax = true;
    let allow_comments = true;

    let options = ParseOptions {
        partial_syntax,
        allow_comments,
        yield_comments: allow_comments,
        ..Default::default()
    };

    let result = parse_advanced::<()>(&content, &options, |keys, value| {
        eprintln!("{keys:?} -> {value:?}");
        None
    });

    if let Err(JSONParseError { at, reason }) = result {
        eprintln!("{reason:?} @ {at}");
    }
}
