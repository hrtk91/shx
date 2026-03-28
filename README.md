# shx - A modern superset of POSIX sh

## What is shx?

shx is a transpiler that takes shell scripts written with clean, brace-based control flow syntax and emits standard POSIX sh. It replaces the ugly parts of shell syntax -- `fi`, `done`, `esac`, `;;` -- with `{}` braces and `=>` arrows, while leaving everything else completely untouched. The output is readable, portable shell script that runs anywhere.

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
```

**Generated POSIX sh**:

```sh
#!/usr/bin/env shx
set -euo pipefail
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
```

## Features

- **`if`/`elif`/`else` with `{}` braces** -- no more `fi`
- **`for`/`while` with `{}` braces** -- no more `do`/`done`
- **`match` with `=>` arrows** -- no more `case`/`esac`/`;;`
- **`_` wildcard** and **`|` alternatives** in match arms
- **Strict mode by default** -- `set -euo pipefail` is auto-injected at the top of every output
- **Everything else is plain sh** -- variables, pipes, redirects, parameter expansion, command substitution all pass through unchanged

## Install

From source (requires Rust toolchain):

```
cargo install --path .
```

## Usage

```sh
# Transpile a file, print to stdout
shx input.shx

# Transpile to a specific output file
shx input.shx -o output.sh

# Read from stdin
cat input.shx | shx

# Pipe directly into sh
shx input.shx | sh
```

## Design philosophy

**Fix what's painful, keep what's worth learning.**

- **Control flow syntax** (`fi`, `done`, `esac`, `;;`) is painful with zero learning value. Every other language uses braces or indentation. Replaced.
- **Parameter expansion** (`${var:-default}`, `$#`, `$@`, etc.) is worth learning. It is powerful, portable, and universal across shells. Kept as-is.
- **Strict mode** (`set -euo pipefail`) should be the default, not an incantation you have to memorize and paste into every script.

## Status

Early and experimental. The transpiler works for the supported constructs, but expect rough edges. Contributions and bug reports are welcome.

## License

[MIT](LICENSE)
