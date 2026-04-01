//! shx formatter — AST を正規化された shx ソースに再出力する。
//!
//! codegen.rs が POSIX sh を出力するのに対し、こちらは shx の波括弧構文を出力する。
//! `set -eu` の注入は行わず、入力をそのまま整形する。

use crate::ast::*;

/// AST を整形された shx ソース文字列に変換する。
pub fn format_shx(nodes: &[Node]) -> String {
    let mut output = String::new();
    format_nodes(nodes, &mut output, 0);
    output
}

fn format_nodes(nodes: &[Node], out: &mut String, indent: usize) {
    for node in nodes {
        format_node(node, out, indent);
    }
}

fn pad(level: usize) -> String {
    "  ".repeat(level)
}

fn format_node(node: &Node, out: &mut String, indent: usize) {
    let p = pad(indent);
    match node {
        Node::Raw(s) => {
            if !s.is_empty() {
                if s.contains('\n') {
                    // ヒアドキュメント等: 最初の行だけインデント
                    let mut lines = s.splitn(2, '\n');
                    out.push_str(&p);
                    out.push_str(lines.next().unwrap());
                    out.push('\n');
                    if let Some(rest) = lines.next() {
                        out.push_str(rest);
                        out.push('\n');
                    }
                } else {
                    out.push_str(&p);
                    out.push_str(s);
                    out.push('\n');
                }
            }
        }
        Node::Comment(c) => {
            out.push_str(&p);
            out.push_str(c);
            out.push('\n');
        }
        Node::If { branches, else_body } => {
            for (i, branch) in branches.iter().enumerate() {
                if i == 0 {
                    out.push_str(&format!("{}if {} {{\n", p, branch.condition));
                } else {
                    out.push_str(&format!("{}}} elif {} {{\n", p, branch.condition));
                }
                format_nodes(&branch.body, out, indent + 1);
            }
            if let Some(body) = else_body {
                out.push_str(&format!("{}}} else {{\n", p));
                format_nodes(body, out, indent + 1);
            }
            out.push_str(&format!("{}}}\n", p));
        }
        Node::For { var, list, body } => {
            out.push_str(&format!("{}for {} in {} {{\n", p, var, list));
            format_nodes(body, out, indent + 1);
            out.push_str(&format!("{}}}\n", p));
        }
        Node::While { condition, body } => {
            out.push_str(&format!("{}while {} {{\n", p, condition));
            format_nodes(body, out, indent + 1);
            out.push_str(&format!("{}}}\n", p));
        }
        Node::Function { name, body } => {
            out.push_str(&format!("{}{}() {{\n", p, name));
            format_nodes(body, out, indent + 1);
            out.push_str(&format!("{}}}\n", p));
        }
        Node::Match { expr, arms } => {
            out.push_str(&format!("{}match {} {{\n", p, expr));
            for arm in arms {
                if arm.body.len() == 1 {
                    if let Node::Raw(s) = &arm.body[0] {
                        out.push_str(&format!("{}  {} => {}\n", p, arm.pattern, s));
                        continue;
                    }
                }
                if arm.body.is_empty() {
                    out.push_str(&format!("{}  {} =>\n", p, arm.pattern));
                } else {
                    out.push_str(&format!("{}  {} => {{\n", p, arm.pattern));
                    format_nodes(&arm.body, out, indent + 2);
                    out.push_str(&format!("{}  }}\n", p));
                }
            }
            out.push_str(&format!("{}}}\n", p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer, parser};

    fn fmt(input: &str) -> String {
        let tokens = lexer::tokenize(input);
        let ast = parser::parse(tokens).unwrap();
        format_shx(&ast)
    }

    #[test]
    fn test_fmt_if_else() {
        let input = "if [ 1 ] {\necho yes\n} else {\necho no\n}";
        let expected = "if [ 1 ] {\n  echo yes\n} else {\n  echo no\n}\n";
        assert_eq!(fmt(input), expected);
    }

    #[test]
    fn test_fmt_for() {
        let input = "for i in 1 2 3 {\necho $i\n}";
        let expected = "for i in 1 2 3 {\n  echo $i\n}\n";
        assert_eq!(fmt(input), expected);
    }

    #[test]
    fn test_fmt_function() {
        let input = "greet() {\necho hello\n}";
        let expected = "greet() {\n  echo hello\n}\n";
        assert_eq!(fmt(input), expected);
    }

    #[test]
    fn test_fmt_match() {
        let input = "match \"$x\" {\n\"a\" => echo a\n_ => echo other\n}";
        let expected = "match \"$x\" {\n  \"a\" => echo a\n  _ => echo other\n}\n";
        assert_eq!(fmt(input), expected);
    }

    #[test]
    fn test_fmt_idempotent() {
        let input = "if [ 1 ] {\n  echo yes\n} else {\n  echo no\n}\n";
        assert_eq!(fmt(input), fmt(&fmt(input)));
    }

    #[test]
    fn test_fmt_shebang_preserved() {
        let input = "#!/usr/bin/env shx\necho hello\n";
        let expected = "#!/usr/bin/env shx\necho hello\n";
        assert_eq!(fmt(input), expected);
    }

    #[test]
    fn test_fmt_nested() {
        let input = "for i in 1 2 {\nif [ 1 ] {\necho $i\n}\n}";
        let expected = "for i in 1 2 {\n  if [ 1 ] {\n    echo $i\n  }\n}\n";
        assert_eq!(fmt(input), expected);
    }
}
