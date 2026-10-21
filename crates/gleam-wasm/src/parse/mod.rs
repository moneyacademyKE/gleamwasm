//! Gleam source parser (subset).
//!
//! Recursive-descent parser for a Gleam subset: functions, `let`, binops,
//! calls, `case`, closures, list literals. All values are typed `I64` and
//! annotations are ignored — see docs/gap_analysis_v2.md §C.

mod expr;
mod lexer;
mod parser;

pub use parser::parse_module;

use lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

fn peek(tokens: &[Token], pos: usize) -> Option<&Token> {
    tokens.get(pos)
}

fn expect(tokens: &[Token], pos: &mut usize, expected: Token) -> Result<(), ParseError> {
    match tokens.get(*pos) {
        Some(tok) if *tok == expected => {
            *pos += 1;
            Ok(())
        }
        _ => Err(error_at(*pos, &format!("expected '{expected:?}'"))),
    }
}

fn expect_ident(tokens: &[Token], pos: &mut usize) -> Result<String, ParseError> {
    match tokens.get(*pos) {
        Some(Token::Ident(name)) => {
            let n = name.clone();
            *pos += 1;
            Ok(n)
        }
        _ => Err(error_at(*pos, "expected identifier")),
    }
}

fn skip_whitespace(tokens: &[Token], pos: &mut usize) {
    while *pos < tokens.len() && matches!(tokens[*pos], Token::Newline) {
        *pos += 1;
    }
}

fn error_at(pos: usize, msg: &str) -> ParseError {
    ParseError {
        message: msg.to_string(),
        line: 0, // simplified
        col: pos,
    }
}
