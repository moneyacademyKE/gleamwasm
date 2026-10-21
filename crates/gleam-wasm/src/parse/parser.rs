//! Module- and statement-level parsing: functions, params, blocks, let.

use super::expr::parse_expression;
use super::lexer::{Token, tokenize};
use super::{ParseError, error_at, expect, expect_ident, peek, skip_whitespace};
use crate::codegen::{GleamFunctionDef, GleamModule, TypedExpr};
use crate::ir::types::ValType;

pub fn parse_module(source: &str) -> Result<GleamModule, ParseError> {
    let mut functions = Vec::new();
    let mut exports = Vec::new();
    let tokens = tokenize(source);
    let mut pos: usize = 0;

    while pos < tokens.len() {
        skip_whitespace(&tokens, &mut pos);
        if pos >= tokens.len() {
            break;
        }

        let is_pub = matches!(peek(&tokens, pos), Some(Token::Pub));
        if is_pub {
            pos += 1;
            skip_whitespace(&tokens, &mut pos);
        }

        if matches!(peek(&tokens, pos), Some(Token::Fn)) {
            pos += 1; // skip 'fn'
            skip_whitespace(&tokens, &mut pos);

            let name = match expect_ident(&tokens, &mut pos) {
                Ok(n) => n,
                Err(_) => return Err(error_at(pos, "expected function name")),
            };

            let params = parse_param_list(&tokens, &mut pos)?;

            // Optional return type annotation
            let return_type = ValType::I64;
            skip_whitespace(&tokens, &mut pos);
            if matches!(peek(&tokens, pos), Some(Token::Arrow)) {
                pos += 1; // skip ->
                skip_whitespace(&tokens, &mut pos);
                skip_type_annotation(&tokens, &mut pos);
            }

            skip_whitespace(&tokens, &mut pos);
            let body = parse_block(&tokens, &mut pos)?;

            functions.push(GleamFunctionDef {
                name: name.clone(),
                params: params.into_iter().map(|p| (p, ValType::I64)).collect(),
                return_type,
                body,
            });
            if is_pub {
                exports.push(name);
            }
        } else if matches!(peek(&tokens, pos), Some(Token::Const)) {
            // Skip const declarations for now
            while pos < tokens.len()
                && !matches!(tokens[pos], Token::Newline)
                && !matches!(tokens[pos], Token::CloseBrace)
            {
                pos += 1;
            }
            skip_whitespace(&tokens, &mut pos);
        } else if matches!(peek(&tokens, pos), Some(Token::Type)) {
            // Skip type declarations for now
            while pos < tokens.len()
                && !matches!(tokens[pos], Token::Newline)
                && !matches!(tokens[pos], Token::CloseBrace)
            {
                pos += 1;
            }
            skip_whitespace(&tokens, &mut pos);
        } else if matches!(peek(&tokens, pos), Some(Token::Import)) {
            // Skip imports for now
            while pos < tokens.len() && !matches!(tokens[pos], Token::Newline) {
                pos += 1;
            }
            skip_whitespace(&tokens, &mut pos);
        } else {
            pos += 1; // skip unrecognized
        }
    }

    Ok(GleamModule {
        functions,
        exports,
        imports: vec![],
        adt_types: vec![],
    })
}

fn skip_type_annotation(tokens: &[Token], pos: &mut usize) {
    let mut depth = 0usize;
    while *pos < tokens.len() {
        match peek(tokens, *pos) {
            Some(Token::OpenBrace) if depth == 0 => break,
            Some(Token::Comma) | Some(Token::CloseParen) if depth == 0 => break,
            Some(Token::OpenParen) => {
                depth += 1;
                *pos += 1;
            }
            Some(Token::CloseParen) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                *pos += 1;
            }
            Some(Token::Newline) if depth == 0 => break,
            Some(_) => *pos += 1,
            None => break,
        }
    }
}

pub(super) fn parse_param_list(
    tokens: &[Token],
    pos: &mut usize,
) -> Result<Vec<String>, ParseError> {
    skip_whitespace(tokens, pos);
    expect(tokens, pos, Token::OpenParen)?;
    skip_whitespace(tokens, pos);

    let mut params = Vec::new();
    while !matches!(peek(tokens, *pos), Some(Token::CloseParen)) {
        if *pos >= tokens.len() {
            return Err(error_at(*pos, "unexpected end of parameter list"));
        }

        // Skip type annotation if present: name: Type
        if matches!(peek(tokens, *pos), Some(Token::Ident(_))) {
            let name = tokens[*pos].as_ident().unwrap().to_string();
            *pos += 1;
            params.push(name);
            // Skip "type" annotation like ": Int" or ": Float"
            skip_whitespace(tokens, pos);
            if matches!(peek(tokens, *pos), Some(Token::Colon)) {
                *pos += 1; // skip ':'
                skip_whitespace(tokens, pos);
                skip_type_annotation(tokens, pos);
            }
        }

        skip_whitespace(tokens, pos);
        if matches!(peek(tokens, *pos), Some(Token::Comma)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
        }
    }
    *pos += 1; // skip ')'
    Ok(params)
}

pub(super) fn parse_block(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    skip_whitespace(tokens, pos);
    expect(tokens, pos, Token::OpenBrace)?;
    skip_whitespace(tokens, pos);

    let mut statements = Vec::new();
    while !matches!(peek(tokens, *pos), Some(Token::CloseBrace)) {
        if *pos >= tokens.len() {
            return Err(error_at(*pos, "unexpected end of block"));
        }
        let stmt = parse_statement(tokens, pos)?;
        statements.push(stmt);
        skip_whitespace(tokens, pos);
        if matches!(peek(tokens, *pos), Some(Token::Newline)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
        }
    }
    *pos += 1; // skip '}'

    // Fold statements into a Let chain, last expression is the result.
    if statements.is_empty() {
        Ok(TypedExpr::Nil)
    } else {
        let mut body = statements.pop().unwrap();
        for stmt in statements.into_iter().rev() {
            body = match stmt {
                TypedExpr::Let {
                    name,
                    value,
                    body: stmt_body,
                    type_,
                } if matches!(*stmt_body, TypedExpr::Nil) => TypedExpr::Let {
                    name,
                    value,
                    body: Box::new(body),
                    type_,
                },
                other => TypedExpr::Let {
                    name: "_".into(),
                    value: Box::new(other),
                    body: Box::new(body),
                    type_: ValType::I64,
                },
            };
        }
        Ok(body)
    }
}

fn parse_statement(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    // let name = expr :: or just expr
    if matches!(peek(tokens, *pos), Some(Token::Let)) {
        *pos += 1; // skip 'let'
        skip_whitespace(tokens, pos);
        // discard pattern (#(a, b)) or just bindings
        if matches!(peek(tokens, *pos), Some(Token::HashParen)) {
            // tuple destructuring: let #(a, b) = ...
            *pos += 1;
            let mut bindings = Vec::new();
            skip_whitespace(tokens, pos);
            while !matches!(peek(tokens, *pos), Some(Token::CloseParen)) {
                if let Some(Token::Ident(name)) = tokens.get(*pos) {
                    bindings.push(name.clone());
                    *pos += 1;
                } else {
                    *pos += 1;
                }
                skip_whitespace(tokens, pos);
                if matches!(peek(tokens, *pos), Some(Token::Comma)) {
                    *pos += 1;
                    skip_whitespace(tokens, pos);
                }
            }
            *pos += 1; // skip ')'
            skip_whitespace(tokens, pos);
            if matches!(peek(tokens, *pos), Some(Token::Equals)) {
                *pos += 1;
            }
            skip_whitespace(tokens, pos);
            let expr = parse_expression(tokens, pos)?;
            if bindings.len() == 1 {
                return Ok(TypedExpr::Let {
                    name: bindings[0].clone(),
                    value: Box::new(expr),
                    body: Box::new(TypedExpr::Nil),
                    type_: ValType::I64,
                });
            }
            // Multi-binding tuple destructure — emit TupleGet for each
            let tuple_type_idx = 0; // simplified
            return Ok(TypedExpr::TupleGet {
                tuple: Box::new(expr),
                type_index: tuple_type_idx,
                element_index: 0,
                type_: ValType::I64,
            });
        }
        // assert pattern
        if matches!(peek(tokens, *pos), Some(Token::Assert)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
            if let Some(Token::Ident(name)) = tokens.get(*pos) {
                do_let(tokens, pos, name)
            } else {
                Err(error_at(*pos, "expected variable name"))
            }
        } else if let Some(Token::Ident(name)) = tokens.get(*pos) {
            do_let(tokens, pos, name)
        } else {
            Err(error_at(*pos, "expected variable name"))
        }
    } else {
        parse_expression(tokens, pos)
    }
}

fn do_let(tokens: &[Token], pos: &mut usize, name: &str) -> Result<TypedExpr, ParseError> {
    let binding_name = if name == "_" {
        "_".to_string()
    } else {
        name.to_string()
    };
    *pos += 1;
    skip_whitespace(tokens, pos);

    // Optional type annotation: name: Type
    if matches!(peek(tokens, *pos), Some(Token::Colon)) {
        *pos += 1;
        skip_whitespace(tokens, pos);
        if matches!(peek(tokens, *pos), Some(Token::Ident(_))) {
            *pos += 1; // skip type name
        }
        skip_whitespace(tokens, pos);
    }

    if matches!(peek(tokens, *pos), Some(Token::Equals)) {
        *pos += 1;
    }
    skip_whitespace(tokens, pos);
    let value = parse_expression(tokens, pos)?;
    Ok(TypedExpr::Let {
        name: binding_name,
        value: Box::new(value),
        body: Box::new(TypedExpr::Nil),
        type_: ValType::I64,
    })
}
