//! shx CLI — コマンドライン引数の処理とスクリプト実行。

use std::io::{self, Read, Write};
use std::process::Command;

use serde::Deserialize;

/// ~/.config/shx/config.toml の構造
#[derive(Debug, Default, Deserialize)]
struct Config {
    /// デフォルトの出力先シェル ("sh" or "bash")
    #[serde(default)]
    shell: Option<String>,
}

/// XDG_CONFIG_HOME/shx/config.toml を読み込む。なければデフォルト。
fn load_config() -> Config {
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
    });
    let path = config_dir.join("shx").join("config.toml");
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("shx: warning: {}: {}", path.display(), e);
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

/// CLI エントリポイント。main.rs / bashx.rs から呼ばれる。
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let config = load_config();

    let opts = parse_args(&args);

    // LSP はstdin/stdoutを通信に使うため、input読み込みの前に分岐
    if opts.lsp {
        tokio::runtime::Runtime::new()
            .expect("failed to create tokio runtime")
            .block_on(crate::lsp::run());
        return;
    }

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

    if opts.fmt {
        let formatted = crate::format_source(&input).unwrap_or_else(|e| {
            eprintln!("shx: {}", e);
            std::process::exit(1);
        });
        io::stdout().write_all(formatted.as_bytes()).unwrap();
        return;
    }

    if opts.check {
        let tokens = crate::lexer::tokenize(&input);
        match crate::parser::parse(tokens) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("shx: {}", e);
                std::process::exit(1);
            }
        }
    }

    // bashモード判定: --bash フラグ / argv[0]が"bashx" / shebangに"bashx" / config
    let invoked_as_bashx = std::path::Path::new(&args[0])
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "bashx");
    let shebang_bashx = input.starts_with("#!") && input.lines().next().is_some_and(|l| l.contains("bashx"));
    let use_bash = opts.bash || invoked_as_bashx || shebang_bashx || config.shell.as_deref() == Some("bash");
    let shell = if use_bash { crate::Shell::Bash } else { crate::Shell::Sh };

    let output = match crate::transpile_with(&input, shell) {
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
        let shell_cmd = if use_bash { "bash" } else { "sh" };
        let status = Command::new(shell_cmd)
            .arg("-c")
            .arg(&output)
            .arg(opts.input_file.unwrap()) // $0 = original script path
            .args(&opts.run_args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("shx: failed to execute {}: {}", shell_cmd, e);
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
    bash: bool,
    fmt: bool,
    lsp: bool,
    run_args: Vec<&'a str>,
}

fn parse_args<'a>(args: &'a [String]) -> Opts<'a> {
    let mut input = None;
    let mut output = None;
    let mut check = false;
    let mut emit = false;
    let mut bash = false;
    let mut fmt = false;
    let mut lsp = false;
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
            "--bash" => {
                bash = true;
            }
            "-h" | "--help" => {
                println!("shx - POSIX sh のモダンなスーパーセット");
                println!();
                println!("if/for/while/match を {{}} ブレース構文で書き、POSIX sh にトランスパイルします。");
                println!("fi, done, esac, ;; といった終端記号を排除し、読みやすいシェルスクリプトを生成します。");
                println!();
                println!("Usage: shx [OPTIONS] [INPUT]");
                println!("       shx fmt [INPUT]");
                println!("       shx lsp");
                println!();
                println!("Commands:");
                println!("  fmt              shx ソースを整形して出力");
                println!("  lsp              LSP サーバーを起動（エディタ連携用）");
                println!();
                println!("Arguments:");
                println!("  [INPUT]          入力ファイル（省略時は stdin から読み込み）");
                println!();
                println!("Options:");
                println!("  -o, --output <FILE>  出力ファイル（省略時は stdout）");
                println!("      --check          構文チェックのみ");
                println!("      --emit           トランスパイル結果を stdout に出力");
                println!("      --bash           bash 向けに出力（bashx コマンドでも可）");
                println!("  -h, --help           ヘルプを表示");
                println!("  -V, --version        バージョンを表示");
                println!();
                println!("Examples:");
                println!("  shx script.shx           トランスパイルして実行");
                println!("  shx --emit script.shx    トランスパイル結果を表示");
                println!("  shx fmt script.shx       ソースを整形");
                println!("  cat script.shx | shx     stdin から読んでトランスパイル");
                println!();
                println!("Syntax:");
                println!("  shx は POSIX sh のスーパーセットです。通常の sh コードはそのまま書けます。");
                println!("  以下の制御構文で {{}} ブレースを使い、POSIX sh に自動変換されます。");
                println!();
                println!("  if / elif / else:");
                println!("    if [ \"$x\" -gt 0 ] {{");
                println!("      echo \"positive\"");
                println!("    }} elif [ \"$x\" -eq 0 ] {{");
                println!("      echo \"zero\"");
                println!("    }} else {{");
                println!("      echo \"negative\"");
                println!("    }}");
                println!();
                println!("  for:");
                println!("    for i in 1 2 3 {{");
                println!("      echo $i");
                println!("    }}");
                println!();
                println!("  while:");
                println!("    while [ \"$n\" -lt 10 ] {{");
                println!("      n=$((n + 1))");
                println!("    }}");
                println!();
                println!("  match (case の代替):");
                println!("    match \"$val\" {{");
                println!("      \"foo\" => echo \"matched foo\"");
                println!("      \"bar\" | \"baz\" => echo \"bar or baz\"");
                println!("      _ => echo \"default\"");
                println!("    }}");
                println!();
                println!("  関数定義:");
                println!("    greet() {{");
                println!("      echo \"hello $1\"");
                println!("    }}");
                println!();
                println!("  set -eu は自動注入されます（bash モードでは set -euo pipefail）。");
                println!("  変数展開・パイプ・リダイレクト等の sh 機能はそのまま使えます。");
                println!();
                println!("詳細: https://github.com/hrtk91/shx");
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
            "fmt" if input.is_none() && !fmt => {
                fmt = true;
            }
            "lsp" if input.is_none() && !lsp => {
                lsp = true;
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
        bash,
        fmt,
        lsp,
        run_args,
    }
}
