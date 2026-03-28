use std::io::{self, Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (input_file, output_file) = parse_args(&args);

    let input = match input_file {
        Some(path) => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("shx: {}: {}", path, e);
            std::process::exit(1);
        }),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).unwrap();
            buf
        }
    };

    let output = shx::transpile(&input);

    match output_file {
        Some(path) => {
            std::fs::write(path, &output).unwrap_or_else(|e| {
                eprintln!("shx: {}: {}", path, e);
                std::process::exit(1);
            });
        }
        None => {
            io::stdout().write_all(output.as_bytes()).unwrap();
        }
    }
}

fn parse_args<'a>(args: &'a [String]) -> (Option<&'a str>, Option<&'a str>) {
    let mut input = None;
    let mut output = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].as_str());
                } else {
                    eprintln!("shx: -o requires an argument");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                println!("Usage: shx [OPTIONS] [INPUT]");
                println!();
                println!("Transpile shx to POSIX sh");
                println!();
                println!("Arguments:");
                println!("  [INPUT]          Input file (reads stdin if omitted)");
                println!();
                println!("Options:");
                println!("  -o, --output <FILE>  Output file (writes stdout if omitted)");
                println!("  -h, --help           Print help");
                println!("  -V, --version        Print version");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("shx {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("shx: unknown option '{}'", arg);
                std::process::exit(1);
            }
            _ => {
                input = Some(args[i].as_str());
            }
        }
        i += 1;
    }

    (input, output)
}
