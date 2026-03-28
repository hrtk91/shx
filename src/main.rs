use std::io::{self, Read, Write};
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let opts = parse_args(&args);

    let input = match opts.input_file {
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

    if opts.check {
        let tokens = shx::lexer::tokenize(&input);
        match shx::parser::parse(tokens) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("shx: {}", e);
                std::process::exit(1);
            }
        }
    }

    let output = match shx::transpile(&input) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("shx: {}", e);
            std::process::exit(1);
        }
    };

    // Decide mode: -o → file output, --emit → stdout, file input → run, stdin → stdout
    if let Some(path) = opts.output_file {
        std::fs::write(path, &output).unwrap_or_else(|e| {
            eprintln!("shx: {}: {}", path, e);
            std::process::exit(1);
        });
    } else if opts.emit || opts.input_file.is_none() {
        io::stdout().write_all(output.as_bytes()).unwrap();
    } else {
        // Default for file input: transpile and execute
        let status = Command::new("sh")
            .arg("-c")
            .arg(&output)
            .args(&opts.run_args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("shx: failed to execute sh: {}", e);
                std::process::exit(1);
            });
        std::process::exit(status.code().unwrap_or(1));
    }
}

struct Opts<'a> {
    input_file: Option<&'a str>,
    output_file: Option<&'a str>,
    check: bool,
    emit: bool,
    run_args: Vec<&'a str>,
}

fn parse_args<'a>(args: &'a [String]) -> Opts<'a> {
    let mut input = None;
    let mut output = None;
    let mut check = false;
    let mut emit = false;
    let mut run_args = Vec::new();
    let mut i = 1;
    let mut after_dashdash = false;

    while i < args.len() {
        if after_dashdash {
            run_args.push(args[i].as_str());
            i += 1;
            continue;
        }
        match args[i].as_str() {
            "--" => {
                after_dashdash = true;
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output = Some(args[i].as_str());
                } else {
                    eprintln!("shx: -o requires an argument");
                    std::process::exit(1);
                }
            }
            "--check" => {
                check = true;
            }
            "--emit" => {
                emit = true;
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
                println!("      --check          Check syntax only");
                println!("      --emit           Output transpiled POSIX sh to stdout");
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

    Opts {
        input_file: input,
        output_file: output,
        check,
        emit,
        run_args,
    }
}
