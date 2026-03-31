//! shx parser — トークン列を AST に変換する。
//!
//! shx 独自の波括弧構文（if/for/while/match/function）を認識し、
//! それ以外の行は `Node::Raw` としてそのまま保持する。

use crate::ast::*;
use crate::lexer::{Span, Token, TokenKind};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.column, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Node>, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse_top()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.peek().map(|t| t.span).unwrap_or(Span {
            line: 1,
            column: 1,
        })
    }

    fn next(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn peek_word(&self) -> Option<&str> {
        match self.peek_kind() {
            Some(TokenKind::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(
            self.peek_kind(),
            Some(TokenKind::Newline | TokenKind::Semicolon)
        ) {
            self.next();
        }
    }

    fn expect_open_brace(&mut self) -> Result<(), ParseError> {
        let span = self.current_span();
        match self.next() {
            Some(Token { kind: TokenKind::OpenBrace, .. }) => Ok(()),
            Some(t) => Err(ParseError {
                message: format!("expected '{{', got {:?}", t.kind),
                span,
            }),
            None => Err(ParseError {
                message: "expected '{', got end of input".into(),
                span,
            }),
        }
    }

    fn expect_close_brace(&mut self) -> Result<(), ParseError> {
        let span = self.current_span();
        match self.next() {
            Some(Token { kind: TokenKind::CloseBrace, .. }) => Ok(()),
            Some(t) => Err(ParseError {
                message: format!("expected '}}', got {:?}", t.kind),
                span,
            }),
            None => Err(ParseError {
                message: "expected '}', got end of input".into(),
                span,
            }),
        }
    }

    fn parse_top(&mut self) -> Result<Vec<Node>, ParseError> {
        self.parse_body(false)
    }

    fn parse_body(&mut self, until_close_brace: bool) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        loop {
            self.skip_terminators();
            match self.peek_kind() {
                None => break,
                Some(TokenKind::CloseBrace) if until_close_brace => break,
                _ => {}
            }
            // Standalone comment on its own line
            if let Some(TokenKind::Comment(c)) = self.peek_kind() {
                let c = c.clone();
                self.next();
                if c.starts_with("#!") {
                    nodes.push(Node::Raw(c));
                } else {
                    nodes.push(Node::Comment(c));
                }
                continue;
            }
            let node = match self.peek_word() {
                Some("if") => self.parse_if()?,
                Some("for") => self.parse_for()?,
                Some("while") => self.parse_while()?,
                Some("match") => self.parse_match()?,
                Some(w) if w.ends_with("()") => self.parse_function()?,
                _ => self.parse_raw_line(),
            };
            if let Node::Raw(ref s) = node {
                if s.is_empty() {
                    continue;
                }
            }
            nodes.push(node);
        }
        Ok(nodes)
    }

    fn collect_until_brace(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            match self.peek_kind() {
                None | Some(TokenKind::OpenBrace) => break,
                Some(TokenKind::Newline | TokenKind::Semicolon) => break,
                Some(TokenKind::Word(w)) => {
                    parts.push(w.clone());
                    self.next();
                }
                Some(TokenKind::Comment(c)) => {
                    parts.push(c.clone());
                    self.next();
                }
                Some(TokenKind::Arrow) => {
                    parts.push("=>".into());
                    self.next();
                }
                Some(TokenKind::CloseBrace) => break,
            }
        }
        parts.join(" ")
    }

    fn parse_if(&mut self) -> Result<Node, ParseError> {
        self.next(); // consume "if"
        let condition = self.collect_until_brace();
        self.expect_open_brace()?;
        let body = self.parse_body(true)?;
        self.expect_close_brace()?;

        let mut branches = vec![Branch { condition, body }];

        loop {
            self.skip_terminators();
            if self.peek_word() != Some("elif") {
                break;
            }
            self.next();
            let condition = self.collect_until_brace();
            self.expect_open_brace()?;
            let body = self.parse_body(true)?;
            self.expect_close_brace()?;
            branches.push(Branch { condition, body });
        }

        let else_body = if self.peek_word() == Some("else") {
            self.next();
            self.expect_open_brace()?;
            let body = self.parse_body(true)?;
            self.expect_close_brace()?;
            Some(body)
        } else {
            None
        };

        Ok(Node::If {
            branches,
            else_body,
        })
    }

    fn parse_for(&mut self) -> Result<Node, ParseError> {
        let for_span = self.current_span();
        self.next(); // consume "for"
        let var = match self.next() {
            Some(Token { kind: TokenKind::Word(w), .. }) => w,
            other => {
                let span = other.as_ref().map(|t| t.span).unwrap_or(for_span);
                return Err(ParseError {
                    message: format!(
                        "expected variable name after 'for', got {:?}",
                        other.map(|t| t.kind)
                    ),
                    span,
                });
            }
        };
        match self.peek_word() {
            Some("in") => {
                self.next();
            }
            _ => {
                return Err(ParseError {
                    message: format!("expected 'in' after 'for {}'", var),
                    span: self.current_span(),
                });
            }
        }
        let list = self.collect_until_brace();
        self.expect_open_brace()?;
        let body = self.parse_body(true)?;
        self.expect_close_brace()?;
        Ok(Node::For { var, list, body })
    }

    fn parse_while(&mut self) -> Result<Node, ParseError> {
        self.next(); // consume "while"
        let condition = self.collect_until_brace();
        self.expect_open_brace()?;
        let body = self.parse_body(true)?;
        self.expect_close_brace()?;
        Ok(Node::While { condition, body })
    }

    fn parse_function(&mut self) -> Result<Node, ParseError> {
        let word = match self.next() {
            Some(Token { kind: TokenKind::Word(w), .. }) => w,
            _ => unreachable!(),
        };
        let name = word.strip_suffix("()").unwrap().to_string();
        self.expect_open_brace()?;
        let body = self.parse_body(true)?;
        self.expect_close_brace()?;
        Ok(Node::Function { name, body })
    }

    fn parse_match(&mut self) -> Result<Node, ParseError> {
        self.next(); // consume "match"
        let expr = self.collect_until_brace();
        self.expect_open_brace()?;

        let mut arms = Vec::new();
        loop {
            self.skip_terminators();
            if matches!(self.peek_kind(), Some(TokenKind::CloseBrace) | None) {
                break;
            }
            arms.push(self.parse_match_arm()?);
        }
        self.expect_close_brace()?;

        Ok(Node::Match { expr, arms })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let mut pattern_parts = Vec::new();
        loop {
            match self.peek_kind() {
                Some(TokenKind::Arrow) => {
                    self.next();
                    break;
                }
                Some(TokenKind::Word(w)) => {
                    pattern_parts.push(w.clone());
                    self.next();
                }
                Some(TokenKind::Newline) | None => {
                    return Err(ParseError {
                        message: "expected '=>' in match arm".into(),
                        span: self.current_span(),
                    });
                }
                _ => {
                    self.next();
                }
            }
        }
        let pattern = pattern_parts.join(" ");

        let body = if matches!(self.peek_kind(), Some(TokenKind::OpenBrace)) {
            self.next();
            let body = self.parse_body(true)?;
            self.expect_close_brace()?;
            body
        } else {
            let line = self.parse_raw_line();
            if let Node::Raw(ref s) = line {
                if s.is_empty() {
                    return Ok(MatchArm {
                        pattern,
                        body: vec![],
                    });
                }
            }
            vec![line]
        };

        Ok(MatchArm { pattern, body })
    }

    fn parse_raw_line(&mut self) -> Node {
        let mut parts = Vec::new();
        loop {
            match self.peek_kind() {
                None => break,
                Some(TokenKind::Newline | TokenKind::Semicolon) => {
                    self.next();
                    break;
                }
                Some(TokenKind::CloseBrace) => break,
                Some(TokenKind::Word(w)) => {
                    parts.push(w.clone());
                    self.next();
                }
                Some(TokenKind::OpenBrace) => {
                    parts.push("{".into());
                    self.next();
                }
                Some(TokenKind::Arrow) => {
                    parts.push("=>".into());
                    self.next();
                }
                Some(TokenKind::Comment(c)) => {
                    parts.push(c.clone());
                    self.next();
                }
            }
        }
        Node::Raw(parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_if() {
        let tokens = tokenize("if [ \"$x\" -gt 0 ] {\n  echo yes\n}");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![Node::If {
                branches: vec![Branch {
                    condition: "[ \"$x\" -gt 0 ]".into(),
                    body: vec![Node::Raw("echo yes".into())],
                }],
                else_body: None,
            }]
        );
    }

    #[test]
    fn test_parse_if_else() {
        let tokens = tokenize("if [ 1 ] {\n  echo a\n} else {\n  echo b\n}");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![Node::If {
                branches: vec![Branch {
                    condition: "[ 1 ]".into(),
                    body: vec![Node::Raw("echo a".into())],
                }],
                else_body: Some(vec![Node::Raw("echo b".into())]),
            }]
        );
    }

    #[test]
    fn test_parse_for() {
        let tokens = tokenize("for i in 1 2 3 {\n  echo $i\n}");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![Node::For {
                var: "i".into(),
                list: "1 2 3".into(),
                body: vec![Node::Raw("echo $i".into())],
            }]
        );
    }

    #[test]
    fn test_parse_while() {
        let tokens = tokenize("while [ \"$n\" -lt 10 ] {\n  n=$((n+1))\n}");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![Node::While {
                condition: "[ \"$n\" -lt 10 ]".into(),
                body: vec![Node::Raw("n=$((n+1))".into())],
            }]
        );
    }

    #[test]
    fn test_parse_match() {
        let tokens = tokenize("match \"$val\" {\n  \"foo\" => echo foo\n  _ => echo default\n}");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![Node::Match {
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
            }]
        );
    }

    #[test]
    fn test_passthrough() {
        let tokens = tokenize("echo hello\nFOO=bar\n");
        let ast = parse(tokens).unwrap();
        assert_eq!(
            ast,
            vec![
                Node::Raw("echo hello".into()),
                Node::Raw("FOO=bar".into()),
            ]
        );
    }

    #[test]
    fn test_error_missing_brace() {
        let tokens = tokenize("if [ 1 ]\n  echo yes\n");
        let err = parse(tokens).unwrap_err();
        assert!(err.message.contains("expected '{'"));
        assert_eq!(err.span.line, 1);
    }

    #[test]
    fn test_error_missing_in() {
        let tokens = tokenize("for i {\n  echo $i\n}");
        let err = parse(tokens).unwrap_err();
        assert!(err.message.contains("expected 'in'"));
    }
}
