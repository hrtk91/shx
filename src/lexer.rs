//! shx lexer — ソース文字列をトークン列に変換する。
//!
//! シェル構文（クォート、パラメータ展開、ヒアドキュメント等）を認識しつつ、
//! shx 独自のトークン（`{` `}` `=>`）を切り出す。

use std::iter::Peekable;
use std::str::Chars;

/// ソース上の位置情報（1-based）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,   // 1-based
    pub column: usize, // 1-based
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Word(String),
    OpenBrace,
    CloseBrace,
    Arrow, // =>
    Newline,
    Semicolon,
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// ソース文字列をトークン列に変換する。
/// ヒアドキュメント・クォート・コマンド置換などのシェル構文を適切に扱う。
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line: usize = 1;
    let mut col: usize = 1;

    // Helper: compute how a consumed string moved (line, col)
    fn advance_pos(s: &str, line: &mut usize, col: &mut usize) {
        for c in s.chars() {
            if c == '\n' {
                *line += 1;
                *col = 1;
            } else {
                *col += 1;
            }
        }
    }

    while let Some(&ch) = chars.peek() {
        let span = Span { line, column: col };
        match ch {
            ' ' | '\t' => {
                chars.next();
                col += 1;
            }
            '\n' => {
                chars.next();
                tokens.push(Token { kind: TokenKind::Newline, span });
                line += 1;
                col = 1;
            }
            '#' => {
                let comment = read_comment(&mut chars);
                advance_pos(&comment, &mut line, &mut col);
                tokens.push(Token { kind: TokenKind::Comment(comment), span });
            }
            '{' => {
                chars.next();
                col += 1;
                tokens.push(Token { kind: TokenKind::OpenBrace, span });
            }
            '}' => {
                chars.next();
                col += 1;
                tokens.push(Token { kind: TokenKind::CloseBrace, span });
            }
            ';' => {
                chars.next();
                col += 1;
                tokens.push(Token { kind: TokenKind::Semicolon, span });
            }
            _ => {
                let word = read_word(&mut chars);
                advance_pos(&word, &mut line, &mut col);
                if word == "=>" {
                    tokens.push(Token { kind: TokenKind::Arrow, span });
                } else {
                    // Detect heredoc: <<DELIM, <<'DELIM', <<-DELIM, <<-'DELIM'
                    let heredoc_delim = extract_heredoc_delimiter(&word);
                    if let Some((delim, strip_tabs)) = heredoc_delim {
                        let mut heredoc = word;
                        heredoc.push('\n');
                        let mut rest_of_line: Vec<(String, Span)> = Vec::new();
                        loop {
                            match chars.peek() {
                                Some(&'\n') | None => break,
                                Some(&' ') | Some(&'\t') => { chars.next(); col += 1; }
                                Some(&'#') => {
                                    let s = Span { line, column: col };
                                    let c = read_comment(&mut chars);
                                    advance_pos(&c, &mut line, &mut col);
                                    rest_of_line.push((c, s));
                                }
                                _ => {
                                    let s = Span { line, column: col };
                                    let w = read_word(&mut chars);
                                    advance_pos(&w, &mut line, &mut col);
                                    rest_of_line.push((w, s));
                                }
                            }
                        }
                        if chars.peek() == Some(&'\n') {
                            chars.next();
                            line += 1;
                            col = 1;
                        }
                        let body = read_heredoc_body(&mut chars, &delim, strip_tabs);
                        advance_pos(&body, &mut line, &mut col);
                        heredoc.push_str(&body);
                        tokens.push(Token { kind: TokenKind::Word(heredoc), span });
                        for (r, s) in rest_of_line {
                            if r == "=>" {
                                tokens.push(Token { kind: TokenKind::Arrow, span: s });
                            } else if r.starts_with('#') {
                                tokens.push(Token { kind: TokenKind::Comment(r), span: s });
                            } else {
                                tokens.push(Token { kind: TokenKind::Word(r), span: s });
                            }
                        }
                    } else {
                        tokens.push(Token { kind: TokenKind::Word(word), span });
                    }
                }
            }
        }
    }

    tokens
}

/// Extract heredoc delimiter from a word like `<<EOF`, `<<'EOF'`, `<<-EOF`.
/// Returns (delimiter, strip_leading_tabs).
fn extract_heredoc_delimiter(word: &str) -> Option<(String, bool)> {
    // Word must contain << (could be part of a larger token like cat<<EOF)
    let heredoc_pos = word.find("<<")?;
    let after = &word[heredoc_pos + 2..];
    if after.is_empty() {
        return None;
    }
    let (after, strip_tabs) = if after.starts_with('-') {
        (&after[1..], true)
    } else {
        (after, false)
    };
    if after.is_empty() {
        return None;
    }
    // Strip quotes: 'DELIM' or "DELIM"
    let delim = if (after.starts_with('\'') && after.ends_with('\''))
        || (after.starts_with('"') && after.ends_with('"'))
    {
        after[1..after.len() - 1].to_string()
    } else {
        after.to_string()
    };
    if delim.is_empty() {
        return None;
    }
    Some((delim, strip_tabs))
}

/// Read heredoc body lines until a line matches the delimiter exactly.
fn read_heredoc_body(chars: &mut Peekable<Chars>, delimiter: &str, strip_tabs: bool) -> String {
    let mut body = String::new();
    loop {
        // Read one line
        let mut line = String::new();
        loop {
            match chars.next() {
                None => {
                    // EOF before delimiter — emit what we have
                    if !line.is_empty() {
                        body.push_str(&line);
                        body.push('\n');
                    }
                    // Remove trailing newline
                    if body.ends_with('\n') {
                        body.pop();
                    }
                    return body;
                }
                Some('\n') => break,
                Some(c) => line.push(c),
            }
        }
        // Check if this line is the delimiter
        let trimmed = if strip_tabs {
            line.trim_start_matches('\t')
        } else {
            &line
        };
        if trimmed == delimiter {
            body.push_str(&line);
            // Remove trailing newline from body
            if body.ends_with('\n') {
                body.pop();
            }
            return body;
        }
        body.push_str(&line);
        body.push('\n');
    }
}

fn read_comment(chars: &mut Peekable<Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = chars.peek() {
        if c == '\n' {
            break;
        }
        s.push(c);
        chars.next();
    }
    s
}

fn read_word(chars: &mut Peekable<Chars>) -> String {
    let mut word = String::new();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | ';' | '#' => break,
            '{' => {
                // ${ is parameter expansion — part of the word
                if word.ends_with('$') {
                    word.push(ch);
                    chars.next();
                    read_until_matching(chars, &mut word, '{', '}');
                    continue;
                }
                break;
            }
            '}' => break,
            '\'' => {
                read_single_quoted(chars, &mut word);
            }
            '"' => {
                read_double_quoted(chars, &mut word);
            }
            '`' => {
                read_backtick(chars, &mut word);
            }
            '\\' => {
                word.push(ch);
                chars.next();
                if let Some(&next) = chars.peek() {
                    word.push(next);
                    chars.next();
                }
            }
            '$' => {
                word.push(ch);
                chars.next();
                if let Some(&next) = chars.peek() {
                    match next {
                        '(' => {
                            word.push(next);
                            chars.next();
                            if chars.peek() == Some(&'(') {
                                // $(( arithmetic ))
                                word.push('(');
                                chars.next();
                                read_until_double_paren(chars, &mut word);
                            } else {
                                // $( command substitution )
                                read_until_matching(chars, &mut word, '(', ')');
                            }
                        }
                        '{' => {
                            word.push(next);
                            chars.next();
                            read_until_matching(chars, &mut word, '{', '}');
                        }
                        // $# $? $@ $* $! $$ $- $0-$9 $VAR
                        '#' | '?' | '@' | '*' | '!' | '-'
                        | '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                            word.push(next);
                            chars.next();
                            // Continue reading alphanumeric for $VAR_NAME
                            if next.is_alphabetic() || next == '_' {
                                while let Some(&c) = chars.peek() {
                                    if c.is_alphanumeric() || c == '_' {
                                        word.push(c);
                                        chars.next();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                word.push(ch);
                chars.next();
            }
        }
    }

    word
}

fn read_single_quoted(chars: &mut Peekable<Chars>, word: &mut String) {
    word.push('\'');
    chars.next();
    while let Some(&c) = chars.peek() {
        word.push(c);
        chars.next();
        if c == '\'' {
            return;
        }
    }
}

fn read_double_quoted(chars: &mut Peekable<Chars>, word: &mut String) {
    word.push('"');
    chars.next();
    while let Some(&c) = chars.peek() {
        word.push(c);
        chars.next();
        match c {
            '"' => return,
            '\\' => {
                if let Some(&next) = chars.peek() {
                    word.push(next);
                    chars.next();
                }
            }
            '$' => {
                if let Some(&next) = chars.peek() {
                    match next {
                        '(' => {
                            word.push(next);
                            chars.next();
                            if chars.peek() == Some(&'(') {
                                word.push('(');
                                chars.next();
                                read_until_double_paren(chars, word);
                            } else {
                                read_until_matching(chars, word, '(', ')');
                            }
                        }
                        '{' => {
                            word.push(next);
                            chars.next();
                            read_until_matching(chars, word, '{', '}');
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn read_backtick(chars: &mut Peekable<Chars>, word: &mut String) {
    word.push('`');
    chars.next();
    while let Some(&c) = chars.peek() {
        word.push(c);
        chars.next();
        match c {
            '`' => return,
            '\\' => {
                if let Some(&next) = chars.peek() {
                    word.push(next);
                    chars.next();
                }
            }
            _ => {}
        }
    }
}

fn read_until_matching(chars: &mut Peekable<Chars>, word: &mut String, open: char, close: char) {
    let mut depth = 1;
    while let Some(&c) = chars.peek() {
        word.push(c);
        chars.next();
        if c == open {
            depth += 1;
        }
        if c == close {
            depth -= 1;
            if depth == 0 {
                return;
            }
        }
        if c == '\'' {
            // Inside single quotes, no nesting
            while let Some(&c2) = chars.peek() {
                word.push(c2);
                chars.next();
                if c2 == '\'' {
                    break;
                }
            }
        }
        if c == '"' {
            read_double_quoted_inner(chars, word);
        }
    }
}

/// Read the rest of a double-quoted string (opening " already consumed)
fn read_double_quoted_inner(chars: &mut Peekable<Chars>, word: &mut String) {
    while let Some(&c) = chars.peek() {
        word.push(c);
        chars.next();
        match c {
            '"' => return,
            '\\' => {
                if let Some(&next) = chars.peek() {
                    word.push(next);
                    chars.next();
                }
            }
            _ => {}
        }
    }
}

fn read_until_double_paren(chars: &mut Peekable<Chars>, word: &mut String) {
    loop {
        match chars.peek() {
            None => return,
            Some(&')') => {
                word.push(')');
                chars.next();
                if chars.peek() == Some(&')') {
                    word.push(')');
                    chars.next();
                    return;
                }
            }
            Some(&c) => {
                word.push(c);
                chars.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn test_if_brace() {
        let tokens = tokenize("if [ \"$x\" -gt 0 ] {\n  echo yes\n}");
        assert_eq!(
            kinds(&tokens),
            vec![
                Word("if".into()),
                Word("[".into()),
                Word("\"$x\"".into()),
                Word("-gt".into()),
                Word("0".into()),
                Word("]".into()),
                OpenBrace,
                Newline,
                Word("echo".into()),
                Word("yes".into()),
                Newline,
                CloseBrace,
            ]
        );
    }

    #[test]
    fn test_arrow() {
        let tokens = tokenize("\"foo\" => echo hello");
        assert_eq!(
            kinds(&tokens),
            vec![
                Word("\"foo\"".into()),
                Arrow,
                Word("echo".into()),
                Word("hello".into()),
            ]
        );
    }

    #[test]
    fn test_parameter_expansion() {
        let tokens = tokenize("echo ${HOME}");
        assert_eq!(
            kinds(&tokens),
            vec![Word("echo".into()), Word("${HOME}".into())]
        );
    }

    #[test]
    fn test_command_substitution() {
        let tokens = tokenize("echo $(date)");
        assert_eq!(
            kinds(&tokens),
            vec![Word("echo".into()), Word("$(date)".into())]
        );
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("echo hi # comment\necho bye");
        assert_eq!(
            kinds(&tokens),
            vec![
                Word("echo".into()),
                Word("hi".into()),
                Comment("# comment".into()),
                Newline,
                Word("echo".into()),
                Word("bye".into()),
            ]
        );
    }

    #[test]
    fn test_single_quotes() {
        let tokens = tokenize("echo 'hello world'");
        assert_eq!(
            kinds(&tokens),
            vec![Word("echo".into()), Word("'hello world'".into())]
        );
    }

    #[test]
    fn test_assignment_with_equals() {
        let tokens = tokenize("FOO=bar");
        assert_eq!(kinds(&tokens), vec![Word("FOO=bar".into())]);
    }

    #[test]
    fn test_span_tracking() {
        let tokens = tokenize("echo hello\nif [ 1 ] {");
        assert_eq!(tokens[0].span, Span { line: 1, column: 1 }); // echo
        assert_eq!(tokens[1].span, Span { line: 1, column: 6 }); // hello
        assert_eq!(tokens[2].span, Span { line: 1, column: 11 }); // \n
        assert_eq!(tokens[3].span, Span { line: 2, column: 1 }); // if
    }
}
