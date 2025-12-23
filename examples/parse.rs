use simple_json_parser::{parse_with_options as parse_json, ParseOptions};

static EXAMPLE: &str = r#"
{
    "hello": "world",
    "parser": {
        "name": "json",
        "public": false,
        "features": ["trailing comma"]
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
                options.top_level_separator = Some("\n");
            }
            if path.ends_with(".jsonc") {
                options.allow_comments = true;
            }
            std::fs::read_to_string(path).unwrap()
        } else {
            EXAMPLE.trim_start().to_owned()
        };
        for arg in args {
            if arg == "--comments" {
                options.allow_comments = true;
                options.yield_comments = true;
            } else if arg == "--partial" {
                options.partial_syntax = true;
            } else {
                eprintln!("unknown arg {arg:?}");
            }
        }

        parse_json::<()>(&source, &options, |keys, value| {
            println!("{keys:?} -> {value:?}");
            None
        })
        .unwrap();
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

            let (commands, content) = source.split_once("\n---").unwrap_or(("", &source));
            for command in commands.lines() {
                match command {
                    "new-line-separated" => {
                        options.top_level_separator = Some("\n");
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

            let result = parse_json::<()>(content, &options, |keys, value| {
                println!("{keys:?} -> {value:?}");
                None
            });
            if let Err(err) = result {
                println!("{err:?}");
            }
            // if let Ok((len, _)) = result {
            //     println!("{item:?}", item=&source[..len]);
            // }
            println!("end");
            buf.clear();
            continue;
        }

        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
}
