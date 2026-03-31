pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod cli;

pub use codegen::Shell;

/// shx ソースを POSIX sh に変換する。
pub fn transpile(input: &str) -> Result<String, parser::ParseError> {
    transpile_with(input, Shell::Sh)
}

/// 出力先シェルを指定して shx ソースを変換する。
pub fn transpile_with(input: &str, shell: Shell) -> Result<String, parser::ParseError> {
    let tokens = lexer::tokenize(input);
    let ast = parser::parse(tokens)?;
    Ok(codegen::emit_with(&ast, shell))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRICT: &str = "set -eu\n";

    #[test]
    fn test_if_elif_else() {
        let input = r#"if [ "$x" -gt 0 ] {
  echo "positive"
} elif [ "$x" -eq 0 ] {
  echo "zero"
} else {
  echo "negative"
}"#;
        let expected = format!(
            r#"{STRICT}if [ "$x" -gt 0 ]; then
  echo "positive"
elif [ "$x" -eq 0 ]; then
  echo "zero"
else
  echo "negative"
fi
"#
        );
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_for_loop() {
        let input = "for i in 1 2 3 {\n  echo $i\n}";
        let expected = format!("{STRICT}for i in 1 2 3; do\n  echo $i\ndone\n");
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_while_loop() {
        let input = "while [ \"$n\" -lt 10 ] {\n  n=$((n + 1))\n}";
        let expected = format!("{STRICT}while [ \"$n\" -lt 10 ]; do\n  n=$((n + 1))\ndone\n");
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_match() {
        let input = r#"match "$val" {
  "foo" => echo "foo"
  "bar" | "baz" => echo "bar or baz"
  _ => echo "default"
}"#;
        let expected = format!(
            r#"{STRICT}case "$val" in
  "foo") echo "foo";;
  "bar"|"baz") echo "bar or baz";;
  *) echo "default";;
esac
"#
        );
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_shebang_passthrough() {
        let input = "#!/bin/sh\necho hello\nFOO=bar\n";
        let expected = "#!/bin/sh\nset -eu\necho hello\nFOO=bar\n";
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_shebang_shx_replaced() {
        let input = "#!/usr/bin/env shx\necho hello\n";
        let expected = "#!/bin/sh\nset -eu\necho hello\n";
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_heredoc() {
        let input = "cat <<EOF\nhello\nworld\nEOF\n";
        let expected = "set -eu\ncat <<EOF\nhello\nworld\nEOF\n";
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_heredoc_in_if() {
        let input = "if [ 1 ] {\n  cat <<EOF\nhello\nEOF\n}";
        let expected = "set -eu\nif [ 1 ]; then\n  cat <<EOF\nhello\nEOF\nfi\n";
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_nested_if_in_for() {
        let input = r#"for f in *.txt {
  if [ -f "$f" ] {
    echo "$f exists"
  }
}"#;
        let expected = format!(
            r#"{STRICT}for f in *.txt; do
  if [ -f "$f" ]; then
    echo "$f exists"
  fi
done
"#
        );
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_comment_preserved() {
        let input = "# this is a comment\necho hello\n";
        let expected = format!("{STRICT}# this is a comment\necho hello\n");
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_comment_in_if() {
        let input = "if [ 1 ] {\n  # inside\n  echo yes\n}";
        let expected = format!("{STRICT}if [ 1 ]; then\n  # inside\n  echo yes\nfi\n");
        assert_eq!(transpile(input).unwrap(), expected);
    }

    #[test]
    fn test_match_multiline_arm() {
        let input = r#"match "$1" {
  "start" => {
    echo "starting"
    run_start
  }
  _ => echo "unknown"
}"#;
        let expected = format!(
            r#"{STRICT}case "$1" in
  "start")
    echo "starting"
    run_start
    ;;
  *) echo "unknown";;
esac
"#
        );
        assert_eq!(transpile(input).unwrap(), expected);
    }
}
