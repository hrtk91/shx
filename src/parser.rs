use crate::ast::*;
use crate::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> Vec<Node> {
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

    fn next(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn peek_word(&self) -> Option<&str> {
        match self.peek() {
            Some(Token::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(self.peek(), Some(Token::Newline | Token::Semicolon)) {
            self.next();
        }
    }

    fn expect_open_brace(&mut self) {
        match self.next() {
            Some(Token::OpenBrace) => {}
            other => panic!("expected '{{', got {:?}", other),
        }
    }

    fn expect_close_brace(&mut self) {
        match self.next() {
            Some(Token::CloseBrace) => {}
            other => panic!("expected '}}', got {:?}", other),
        }
    }

    fn parse_top(&mut self) -> Vec<Node> {
        self.parse_body(false)
    }

    fn parse_body(&mut self, until_close_brace: bool) -> Vec<Node> {
        let mut nodes = Vec::new();
        loop {
            self.skip_terminators();
            match self.peek() {
                None => break,
                Some(Token::CloseBrace) if until_close_brace => break,
                _ => {}
            }
            let node = match self.peek_word() {
                Some("if") => self.parse_if(),
                Some("for") => self.parse_for(),
                Some("while") => self.parse_while(),
                Some("match") => self.parse_match(),
                _ => self.parse_raw_line(),
            };
            // Skip empty raw nodes
            if let Node::Raw(ref s) = node {
                if s.is_empty() {
                    continue;
                }
            }
            nodes.push(node);
        }
        nodes
    }

    /// Collect words until `{`, returning them joined by spaces.
    fn collect_until_brace(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            match self.peek() {
                None | Some(Token::OpenBrace) => break,
                Some(Token::Newline | Token::Semicolon) => break,
                Some(Token::Word(w)) => {
                    parts.push(w.clone());
                    self.next();
                }
                Some(Token::Comment(c)) => {
                    parts.push(c.clone());
                    self.next();
                }
                Some(Token::Arrow) => {
                    parts.push("=>".into());
                    self.next();
                }
                Some(Token::CloseBrace) => break,
            }
        }
        parts.join(" ")
    }

    fn parse_if(&mut self) -> Node {
        self.next(); // consume "if"
        let condition = self.collect_until_brace();
        self.expect_open_brace();
        let body = self.parse_body(true);
        self.expect_close_brace();

        let mut branches = vec![Branch { condition, body }];

        // elif branches
        loop {
            self.skip_terminators();
            if self.peek_word() != Some("elif") {
                break;
            }
            self.next(); // consume "elif"
            let condition = self.collect_until_brace();
            self.expect_open_brace();
            let body = self.parse_body(true);
            self.expect_close_brace();
            branches.push(Branch { condition, body });
        }

        // else
        let else_body = if self.peek_word() == Some("else") {
            self.next();
            self.expect_open_brace();
            let body = self.parse_body(true);
            self.expect_close_brace();
            Some(body)
        } else {
            None
        };

        Node::If {
            branches,
            else_body,
        }
    }

    fn parse_for(&mut self) -> Node {
        self.next(); // consume "for"
        let var = match self.next() {
            Some(Token::Word(w)) => w,
            other => panic!("expected variable name after 'for', got {:?}", other),
        };
        match self.peek_word() {
            Some("in") => {
                self.next();
            }
            other => panic!("expected 'in' after 'for {var}', got {:?}", other),
        }
        let list = self.collect_until_brace();
        self.expect_open_brace();
        let body = self.parse_body(true);
        self.expect_close_brace();
        Node::For { var, list, body }
    }

    fn parse_while(&mut self) -> Node {
        self.next(); // consume "while"
        let condition = self.collect_until_brace();
        self.expect_open_brace();
        let body = self.parse_body(true);
        self.expect_close_brace();
        Node::While { condition, body }
    }

    fn parse_match(&mut self) -> Node {
        self.next(); // consume "match"
        let expr = self.collect_until_brace();
        self.expect_open_brace();

        let mut arms = Vec::new();
        loop {
            self.skip_terminators();
            if matches!(self.peek(), Some(Token::CloseBrace) | None) {
                break;
            }
            arms.push(self.parse_match_arm());
        }
        self.expect_close_brace();

        Node::Match { expr, arms }
    }

    fn parse_match_arm(&mut self) -> MatchArm {
        // Collect pattern until =>
        let mut pattern_parts = Vec::new();
        loop {
            match self.peek() {
                Some(Token::Arrow) => {
                    self.next();
                    break;
                }
                Some(Token::Word(w)) => {
                    pattern_parts.push(w.clone());
                    self.next();
                }
                Some(Token::Newline) | None => {
                    panic!("expected '=>' in match arm, got {:?}", self.peek());
                }
                _ => {
                    self.next();
                }
            }
        }
        let pattern = pattern_parts.join(" ");

        // Body: if next is {, parse block; otherwise single line
        let body = if matches!(self.peek(), Some(Token::OpenBrace)) {
            self.next(); // consume {
            let body = self.parse_body(true);
            self.expect_close_brace();
            body
        } else {
            let line = self.parse_raw_line();
            if let Node::Raw(ref s) = line {
                if s.is_empty() {
                    return MatchArm {
                        pattern,
                        body: vec![],
                    };
                }
            }
            vec![line]
        };

        MatchArm { pattern, body }
    }

    fn parse_raw_line(&mut self) -> Node {
        let mut parts = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(Token::Newline | Token::Semicolon) => {
                    self.next();
                    break;
                }
                Some(Token::CloseBrace) => break, // don't consume — caller handles it
                Some(Token::Word(w)) => {
                    parts.push(w.clone());
                    self.next();
                }
                Some(Token::OpenBrace) => {
                    parts.push("{".into());
                    self.next();
                }
                Some(Token::Arrow) => {
                    parts.push("=>".into());
                    self.next();
                }
                Some(Token::Comment(c)) => {
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
        let ast = parse(tokens);
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
        let ast = parse(tokens);
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
        let ast = parse(tokens);
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
        let ast = parse(tokens);
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
        let ast = parse(tokens);
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
        let ast = parse(tokens);
        assert_eq!(
            ast,
            vec![
                Node::Raw("echo hello".into()),
                Node::Raw("FOO=bar".into()),
            ]
        );
    }
}
