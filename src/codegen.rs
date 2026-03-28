use crate::ast::*;

pub fn emit(nodes: &[Node]) -> String {
    let mut output = String::new();

    // shx strict mode: inject set -euo pipefail after shebang (if any)
    let (shebang, rest) = match nodes.first() {
        Some(Node::Raw(s)) if s.starts_with("#!") => {
            output.push_str(s);
            output.push('\n');
            output.push_str("set -euo pipefail\n");
            (true, &nodes[1..])
        }
        _ => {
            output.push_str("set -euo pipefail\n");
            (false, nodes)
        }
    };
    let _ = shebang;

    emit_nodes(rest, &mut output, 0);
    output
}

fn emit_nodes(nodes: &[Node], out: &mut String, indent: usize) {
    for node in nodes {
        emit_node(node, out, indent);
    }
}

fn pad(level: usize) -> String {
    "  ".repeat(level)
}

fn emit_node(node: &Node, out: &mut String, indent: usize) {
    let p = pad(indent);
    match node {
        Node::Raw(s) => {
            if !s.is_empty() {
                out.push_str(&p);
                out.push_str(s);
                out.push('\n');
            }
        }
        Node::If {
            branches,
            else_body,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                if i == 0 {
                    out.push_str(&format!("{}if {}; then\n", p, branch.condition));
                } else {
                    out.push_str(&format!("{}elif {}; then\n", p, branch.condition));
                }
                emit_nodes(&branch.body, out, indent + 1);
            }
            if let Some(body) = else_body {
                out.push_str(&format!("{}else\n", p));
                emit_nodes(body, out, indent + 1);
            }
            out.push_str(&format!("{}fi\n", p));
        }
        Node::For { var, list, body } => {
            out.push_str(&format!("{}for {} in {}; do\n", p, var, list));
            emit_nodes(body, out, indent + 1);
            out.push_str(&format!("{}done\n", p));
        }
        Node::While { condition, body } => {
            out.push_str(&format!("{}while {}; do\n", p, condition));
            emit_nodes(body, out, indent + 1);
            out.push_str(&format!("{}done\n", p));
        }
        Node::Match { expr, arms } => {
            out.push_str(&format!("{}case {} in\n", p, expr));
            for arm in arms {
                let pattern = convert_pattern(&arm.pattern);
                // Single-line arm
                if arm.body.len() == 1 {
                    if let Node::Raw(s) = &arm.body[0] {
                        out.push_str(&format!("{}  {}) {};;\n", p, pattern, s));
                        continue;
                    }
                }
                // Multi-line arm
                out.push_str(&format!("{}  {})\n", p, pattern));
                emit_nodes(&arm.body, out, indent + 2);
                out.push_str(&format!("{}    ;;\n", p));
            }
            out.push_str(&format!("{}esac\n", p));
        }
    }
}

/// Convert shx match pattern to POSIX case pattern.
/// `_` (standalone) becomes `*`.
fn convert_pattern(pattern: &str) -> String {
    pattern
        .split('|')
        .map(|p| {
            let trimmed = p.trim();
            if trimmed == "_" {
                "*".to_string()
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_pattern_wildcard() {
        assert_eq!(convert_pattern("_"), "*");
    }

    #[test]
    fn test_convert_pattern_alternatives() {
        assert_eq!(convert_pattern("\"foo\" | \"bar\""), "\"foo\"|\"bar\"");
    }

    #[test]
    fn test_convert_pattern_wildcard_in_alternatives() {
        assert_eq!(convert_pattern("\"foo\" | _"), "\"foo\"|*");
    }

    const S: &str = "set -euo pipefail\n";

    #[test]
    fn test_emit_raw() {
        let nodes = vec![Node::Raw("echo hello".into())];
        assert_eq!(emit(&nodes), format!("{S}echo hello\n"));
    }

    #[test]
    fn test_emit_if() {
        let nodes = vec![Node::If {
            branches: vec![Branch {
                condition: "[ \"$x\" -gt 0 ]".into(),
                body: vec![Node::Raw("echo yes".into())],
            }],
            else_body: None,
        }];
        assert_eq!(
            emit(&nodes),
            format!("{S}if [ \"$x\" -gt 0 ]; then\n  echo yes\nfi\n")
        );
    }

    #[test]
    fn test_emit_for() {
        let nodes = vec![Node::For {
            var: "i".into(),
            list: "1 2 3".into(),
            body: vec![Node::Raw("echo $i".into())],
        }];
        assert_eq!(emit(&nodes), format!("{S}for i in 1 2 3; do\n  echo $i\ndone\n"));
    }

    #[test]
    fn test_emit_match() {
        let nodes = vec![Node::Match {
            expr: "\"$val\"".into(),
            arms: vec![
                MatchArm {
                    pattern: "\"foo\"".into(),
                    body: vec![Node::Raw("echo foo".into())],
                },
                MatchArm {
                    pattern: "_".into(),
                    body: vec![Node::Raw("echo default".into())],
                },
            ],
        }];
        assert_eq!(
            emit(&nodes),
            format!("{S}case \"$val\" in\n  \"foo\") echo foo;;\n  *) echo default;;\nesac\n")
        );
    }
}
