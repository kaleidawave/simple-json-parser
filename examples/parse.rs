use simple_json_parser::{parse_with_options as parse_json, ParseOptions};

static EXAMPLE: &str = r#"
{
    "hello": "world",
    "parser": {
        "name": "json",
        "public": false,
        "features": ["comments"]
    }
}"#;

fn main() {
    let mut args = std::env::args().skip(1);

    let arg = args.next();

    if let Some("--interactive") = arg.as_deref() {
        run_interactive();
    } else {
        let mut options = ParseOptions::default();
        let source = if let Some("--content") = arg.as_deref() {
            std::env::args().nth(2).expect("no content")
        } else if let Some(path) = arg {
            if path.ends_with(".jsonl") {
                options.top_level_separator = Some('\n');
                // options.top_level_separator = Some("\n");
            }
            if path.ends_with(".jsonc") {
                options.allow_comments = true;
            }
            std::fs::read_to_string(path).unwrap()
        } else {
            EXAMPLE.trim_start().to_owned()
        };
        let mut black_box = false;
        for arg in args {
            if arg == "--comments" {
                options.allow_comments = true;
                options.yield_comments = true;
            } else if arg == "--partial" {
                options.partial_syntax = true;
            } else if arg == "--trailing" {
                options.allow_trailing_commas = true;
            } else if arg == "--black-box" {
                black_box = true;
            } else {
                eprintln!("unknown arg {arg:?}");
            }
        }

        let result = if black_box {
            parse_json::<()>(&source, &options, |keys, value| {
                std::hint::black_box(keys);
                std::hint::black_box(value);
                None
            })
        } else {
            parse_json::<()>(&source, &options, |keys, value| {
                println!("{keys:?} -> {value:?}");
                None
            })
        };

        match result {
            Ok((bytes, value)) => {
                if bytes != source.len() {
                    let rest = &source[bytes..];
                    if !rest.trim().is_empty() {
                        let item = Limit::new(rest);
                        let value = With::new(value);
                        println!("completed before end of source {item}{value:?}");
                    }
                }
            }
            Err(err) => {
                let rest = &source[err.at..];
                let reason = err.reason;
                let item = Limit::new(rest);
                println!("error at {item}, reason {reason:?}");
            }
        }
    }
}

fn run_interactive() {
    use std::io::{stdin, BufRead};
    let stdin = stdin();
    let mut buf = Vec::new();

    println!("start");

    for line in stdin.lock().lines().map_while(Result::ok) {
        if line == "close" {
            if !buf.is_empty() {
                eprintln!("no end to message {buf:?}");
            }
            break;
        }

        if line == "end" {
            let source = String::from_utf8_lossy(&buf);
            let source = source.trim();
            let mut options = ParseOptions::default();

            let (commands, source) = source.split_once("\n---").unwrap_or(("", &source));
            for command in commands.lines() {
                match command {
                    "new-line-separated" => {
                        options.top_level_separator = Some('\n');
                        // options.top_level_separator = Some("\n");
                    }
                    "trailing-commas" => {
                        options.allow_trailing_commas = true;
                    }
                    "with-comments" => {
                        options.allow_comments = true;
                        options.yield_comments = true;
                    }
                    "partial" => {
                        options.partial_syntax = true;
                    }
                    command => {
                        panic!("unknown {command}")
                    }
                }
            }

            let result = parse_json::<&str>(source, &options, |keys, value| {
                use simple_json_parser::RootJSONValue;

                if matches!(value, RootJSONValue::String(value) if value.raw() == "EARLY RETURN") {
                    Some("returned early")
                } else {
                    print!("{keys:?} -> ");
                    match value {
                        RootJSONValue::String(value) => {
                            print!("String({value:?})", value = value.value())
                        }
                        RootJSONValue::Number(value) => {
                            print!("Number({value})", value = value.value_unwrap())
                        }
                        RootJSONValue::Boolean(value) => print!("Boolean({value:?})"),
                        RootJSONValue::Null => print!("Null"),
                        RootJSONValue::EmptyObject => print!("EmptyObject"),
                        RootJSONValue::EmptyArray => print!("EmptyArray"),
                        RootJSONValue::Comment(comment) => print!("Comment({comment:?})"),
                        RootJSONValue::Empty => print!("Empty"),
                    }
                    println!();
                    None
                }
            });
            match result {
                Ok((bytes, value)) => {
                    if bytes != source.len() {
                        let rest = &source[bytes..];
                        if !rest.trim().is_empty() {
                            let item = Limit::new(rest);
                            let value = With::new(value);
                            println!("completed before end of source {item}{value:?}");
                        }
                    }
                }
                Err(err) => {
                    let rest = &source[err.at..];
                    let reason = err.reason;
                    let item = Limit::new(rest);
                    println!("error at {item}, reason {reason:?}");
                }
            }
            println!("end");
            buf.clear();
            continue;
        }

        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
}

struct Limit<'a>(&'a str);

impl<'a> Limit<'a> {
    pub fn new(on: &'a str) -> Self {
        Self(on)
    }
}

impl<'a> std::fmt::Display for Limit<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO customisable
        const LIMIT: usize = 10;

        let on = self.0;
        if let Some(on) = on.get(..LIMIT) {
            write!(fmt, "{on:?}...")
        } else {
            write!(fmt, "{on:?}")
        }
    }
}

struct With<T>(Option<T>);

impl<T> With<T> {
    pub fn new(on: Option<T>) -> Self {
        Self(on)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for With<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(on) = &self.0 {
            write!(fmt, " with {on:?}")
        } else {
            Ok(())
        }
    }
}
