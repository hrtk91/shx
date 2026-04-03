# shx - A modern superset of POSIX sh

## What is shx?

shx is a transpiler that takes shell scripts written with clean, brace-based control flow syntax and emits standard POSIX sh (or bash). It replaces the ugly parts of shell syntax -- `fi`, `done`, `esac`, `;;` -- with `{}` braces and `=>` arrows, while leaving everything else completely untouched. The output is readable, portable shell script that runs anywhere.

## Quick comparison

**shx input** (`demo.shx`):

```sh
#!/usr/bin/env shx
name="world"

if [ "$name" = "world" ] {
  echo "Hello, $name!"
} else {
  echo "Who are you?"
}

for i in 1 2 3 {
  echo "count: $i"
}

while [ "$n" -lt 3 ] {
  n=$((n + 1))
  echo "n=$n"
}

match "$1" {
  "start" => echo "Starting..."
  "stop" | "halt" => echo "Stopping..."
  _ => echo "Usage: $0 {start|stop}"
}

greet() {
  echo "Hello, $1!"
}
greet "$name"
```

**Generated POSIX sh**:

```sh
#!/bin/sh
set -eu
name="world"

if [ "$name" = "world" ]; then
  echo "Hello, $name!"
else
  echo "Who are you?"
fi

for i in 1 2 3; do
  echo "count: $i"
done

while [ "$n" -lt 3 ]; do
  n=$((n + 1))
  echo "n=$n"
done

case "$1" in
  "start") echo "Starting...";;
  "stop"|"halt") echo "Stopping...";;
  *) echo "Usage: $0 {start|stop}";;
esac

greet() {
  echo "Hello, $1!"
}
greet "$name"
```

## Features

- **`if`/`elif`/`else` with `{}` braces** -- no more `fi`
- **`for`/`while` with `{}` braces** -- no more `do`/`done`
- **`match` with `=>` arrows** -- no more `case`/`esac`/`;;`
- **`_` wildcard** and **`|` alternatives** in match arms
- **Function definitions** -- `name() { ... }` works naturally
- **Strict mode by default** -- `set -eu` is auto-injected
- **`--bash` mode** -- target bash instead of POSIX sh
- **Config file** -- set defaults in `~/.config/shx/config.toml`
- **Everything else is plain sh** -- variables, pipes, redirects, parameter expansion, command substitution all pass through unchanged

## Install

ワンライナー:

```sh
curl -fsSL https://raw.githubusercontent.com/hrtk91/shx/master/install.sh | sh
```

[GitHub Releases](https://github.com/hrtk91/shx/releases) からバイナリを直接ダウンロードもできます。

ソースからビルド (Rust toolchain が必要):

```
cargo install --path .
```

## Usage

```sh
# Transpile and execute a file
shx script.shx

# Transpile, print to stdout
shx --emit script.shx

# Transpile to a specific output file
shx script.shx -o output.sh

# Read from stdin, emit to stdout
cat script.shx | shx

# Target bash instead of POSIX sh
shx --bash --emit script.shx

# Syntax check only
shx --check script.shx
```

## Configuration

Create `~/.config/shx/config.toml` to set defaults:

```toml
# Default target shell: "sh" (default) or "bash"
shell = "bash"
```

The `--bash` flag on the command line always takes precedence over the config.

## Design philosophy

**Fix what's painful, keep what's worth learning.**

- **Control flow syntax** (`fi`, `done`, `esac`, `;;`) is painful with zero learning value. Every other language uses braces or indentation. Replaced.
- **Parameter expansion** (`${var:-default}`, `$#`, `$@`, etc.) is worth learning. It is powerful, portable, and universal across shells. Kept as-is.
- **Strict mode** (`set -eu`) should be the default, not an incantation you have to memorize and paste into every script.

## License

[MIT](LICENSE)
