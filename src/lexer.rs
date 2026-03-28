use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    OpenBrace,
    CloseBrace,
    Arrow, // =>
    Newline,
    Semicolon,
    Comment(String),
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' => {
                chars.next();
            }
            '\n' => {
                chars.next();
                tokens.push(Token::Newline);
            }
            '#' => {
                let comment = read_comment(&mut chars);
                tokens.push(Token::Comment(comment));
            }
            '{' => {
                chars.next();
                tokens.push(Token::OpenBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::CloseBrace);
            }
            ';' => {
                chars.next();
                tokens.push(Token::Semicolon);
            }
            _ => {
                let word = read_word(&mut chars);
                if word == "=>" {
                    tokens.push(Token::Arrow);
                } else {
                    // Detect heredoc: <<DELIM, <<'DELIM', <<-DELIM, <<-'DELIM'
                    let heredoc_delim = extract_heredoc_delimiter(&word);
                    if let Some((delim, strip_tabs)) = heredoc_delim {
                        // Build the full heredoc as a single word: <<EOF\nbody\nEOF
                        let mut heredoc = word;
                        heredoc.push('\n');
                        // Consume the newline after the command line
                        // First, read rest of line into separate tokens
                        let mut rest_of_line = Vec::new();
                        loop {
                            match chars.peek() {
                                Some(&'\n') | None => break,
                                Some(&' ') | Some(&'\t') => { chars.next(); }
                                Some(&'#') => {
                                    rest_of_line.push(read_comment(&mut chars));
                                }
                                _ => {
                                    rest_of_line.push(read_word(&mut chars));
                                }
                            }
                        }
                        if chars.peek() == Some(&'\n') {
                            chars.next();
                        }
                        let body = read_heredoc_body(&mut chars, &delim, strip_tabs);
                        heredoc.push_str(&body);
                        tokens.push(Token::Word(heredoc));
                        // Push rest-of-line tokens after the heredoc word
                        for r in rest_of_line {
                            if r == "=>" {
                                tokens.push(Token::Arrow);
                            } else if r.starts_with('#') {
                                tokens.push(Token::Comment(r));
                            } else {
                                tokens.push(Token::Word(r));
                            }
                        }
                    } else {
                        tokens.push(Token::Word(word));
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
    use Token::*;

    #[test]
    fn test_if_brace() {
        let tokens = tokenize("if [ \"$x\" -gt 0 ] {\n  echo yes\n}");
        assert_eq!(
            tokens,
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
            tokens,
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
            tokens,
            vec![Word("echo".into()), Word("${HOME}".into())]
        );
    }

    #[test]
    fn test_command_substitution() {
        let tokens = tokenize("echo $(date)");
        assert_eq!(
            tokens,
            vec![Word("echo".into()), Word("$(date)".into())]
        );
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("echo hi # comment\necho bye");
        assert_eq!(
            tokens,
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
            tokens,
            vec![Word("echo".into()), Word("'hello world'".into())]
        );
    }

    #[test]
    fn test_assignment_with_equals() {
        let tokens = tokenize("FOO=bar");
        assert_eq!(tokens, vec![Word("FOO=bar".into())]);
    }
}
