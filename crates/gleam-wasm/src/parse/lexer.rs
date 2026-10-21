//! Tokenizer: Gleam source chars -> Token stream.

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Fn,
    Pub,
    Let,
    Case,
    Const,
    Type,
    Import,
    As,
    When,
    Assert,
    True,
    False,
    Nil,
    Arrow,
    Pipe,
    Equals,
    Colon,
    Comma,
    Dot,
    DotDot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    HashParen,
    Underscore,
    Newline,
    Ident(String),
    IntLiteral(i32),
    FloatLiteral(f64),
    StringLiteral(String),
}

impl Token {
    pub(super) fn as_ident(&self) -> Option<&String> {
        match self {
            Token::Ident(s) => Some(s),
            _ => None,
        }
    }
}

pub(super) fn tokenize(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '\n' => {
                tokens.push(Token::Newline);
                i += 1;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                // Line comment
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::OpenParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::CloseParen);
                i += 1;
            }
            '{' => {
                tokens.push(Token::OpenBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Token::CloseBrace);
                i += 1;
            }
            '[' => {
                tokens.push(Token::OpenBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::CloseBracket);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::EqEq);
                    i += 2;
                } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Arrow);
                    i += 2;
                } else {
                    tokens.push(Token::Equals);
                    i += 1;
                }
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Arrow);
                    i += 2;
                } else {
                    tokens.push(Token::Minus);
                    i += 1;
                }
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '%' => {
                tokens.push(Token::Percent);
                i += 1;
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::NotEq);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '<' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Le);
                    i += 2;
                } else {
                    tokens.push(Token::Lt);
                    i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    tokens.push(Token::Ge);
                    i += 2;
                } else {
                    tokens.push(Token::Gt);
                    i += 1;
                }
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Pipe);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1] == '.' {
                    tokens.push(Token::DotDot);
                    i += 2;
                } else {
                    tokens.push(Token::Dot);
                    i += 1;
                }
            }
            '#' => {
                if i + 1 < chars.len() && chars[i + 1] == '(' {
                    tokens.push(Token::HashParen);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            '_' => {
                if i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
                    let (ident, next) = read_identifier(&chars, i);
                    tokens.push(Token::Ident(ident));
                    i = next;
                } else {
                    tokens.push(Token::Underscore);
                    i += 1;
                }
            }
            '"' => {
                let (s, next) = read_string(&chars, i);
                tokens.push(Token::StringLiteral(s));
                i = next;
            }
            c if c.is_ascii_digit() => {
                let (num_str, next) = read_number(&chars, i);
                if num_str.contains('.') {
                    tokens.push(Token::FloatLiteral(num_str.parse().unwrap_or(0.0)));
                } else {
                    tokens.push(Token::IntLiteral(num_str.parse().unwrap_or(0)));
                }
                i = next;
            }
            c if c.is_alphabetic()
                || c == '_' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() =>
            {
                let (ident, next) = read_identifier(&chars, i);
                let token = match ident.as_str() {
                    "fn" => Token::Fn,
                    "pub" => Token::Pub,
                    "let" => Token::Let,
                    "case" => Token::Case,
                    "const" => Token::Const,
                    "type" => Token::Type,
                    "import" => Token::Import,
                    "as" => Token::As,
                    "when" => Token::When,
                    "assert" => Token::Assert,
                    "true" => Token::True,
                    "false" => Token::False,
                    "Nil" | "nil" => Token::Nil,
                    _ => Token::Ident(ident),
                };
                tokens.push(token);
                i = next;
            }
            _ => {
                i += 1;
            }
        }
    }
    tokens
}

fn read_identifier(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    while i < chars.len()
        && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '?' || chars[i] == '!')
    {
        i += 1;
    }
    let ident: String = chars[start..i].iter().collect();
    (ident, i)
}

fn read_number(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    let mut has_dot = false;
    while i < chars.len()
        && (chars[i].is_ascii_digit()
            || (chars[i] == '.'
                && !has_dot
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit()))
    {
        if chars[i] == '.' {
            has_dot = true;
        }
        i += 1;
    }
    let num: String = chars[start..i].iter().collect();
    (num, i)
}

fn read_string(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start + 1; // skip opening "
    let mut s = String::new();
    while i < chars.len() && chars[i] != '"' {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 1;
        }
        s.push(chars[i]);
        i += 1;
    }
    (s, i + 1) // skip closing "
}
