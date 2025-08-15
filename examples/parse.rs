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
    let arg = std::env::args().nth(1);

    if let Some("--interactive") = arg.as_deref() {
        run_interactive();
        return;
    }

    let source = if let Some("--content") = arg.as_deref() {
        std::env::args().nth(2).expect("no content")
    } else if let Some(path) = arg {
        std::fs::read_to_string(path).unwrap()
    } else {
        EXAMPLE.trim_start().to_owned()
    };

    let options = ParseOptions::default();

    parse_json::<()>(&source, &options, |keys, value| {
        eprintln!("{keys:?} -> {value:?}");
        None
    })
    .unwrap();
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
            let mut source = source.trim();
            let mut options = ParseOptions::default();
            if let Some(rest) = source.strip_prefix("new-line-separated") {
                options.top_level_separator = Some("\n");
                source = rest.trim_start();
            } else if let Some(rest) = source.strip_prefix("trailing-commas") {
                options.allow_trailing_commas = true;
                source = rest.trim_start();
            } else if let Some(rest) = source.strip_prefix("with-comments") {
                options.allow_comments = true;
                source = rest.trim_start();
            }

            let result = parse_json::<()>(source, &options, |keys, value| {
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
