//! Expression-level parsing: binops, calls, literals, lists, case, closures.

use super::lexer::Token;
use super::parser::{parse_block, parse_param_list};
use super::{ParseError, error_at, expect, peek, skip_whitespace};
use crate::codegen::TypedExpr;
use crate::ir::types::ValType;

pub(super) fn parse_expression(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    parse_binop(tokens, pos)
}

fn parse_binop(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    let mut left = parse_call(tokens, pos)?;
    skip_whitespace(tokens, pos);

    while let Some(op) = binop_token(peek(tokens, *pos)) {
        *pos += 1;
        skip_whitespace(tokens, pos);
        let right = parse_call(tokens, pos)?;
        left = TypedExpr::BinOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
            type_: ValType::I64,
        };
        skip_whitespace(tokens, pos);
    }
    Ok(left)
}

fn parse_call(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    let mut expr = parse_primary(tokens, pos)?;
    skip_whitespace(tokens, pos);
    // Function call: ident(args)
    while matches!(peek(tokens, *pos), Some(Token::OpenParen)) {
        *pos += 1; // skip '('
        skip_whitespace(tokens, pos);
        let mut args = Vec::new();
        while !matches!(peek(tokens, *pos), Some(Token::CloseParen)) {
            if *pos >= tokens.len() {
                return Err(error_at(*pos, "unexpected end of arguments"));
            }
            args.push(parse_expression(tokens, pos)?);
            skip_whitespace(tokens, pos);
            if matches!(peek(tokens, *pos), Some(Token::Comma)) {
                *pos += 1;
                skip_whitespace(tokens, pos);
            }
        }
        *pos += 1; // skip ')'

        // expr is the callee
        if let TypedExpr::Var { name, .. } = &expr {
            let func_name = name.clone();
            let callee = TypedExpr::Call {
                name: func_name,
                args,
                type_: ValType::I64,
            };
            expr = callee;
        } else {
            // Method-style call on non-var — not supported yet
            expr = TypedExpr::Call {
                name: "_unknown".into(),
                args,
                type_: ValType::I64,
            };
        }
        skip_whitespace(tokens, pos);
    }
    Ok(expr)
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    match tokens.get(*pos) {
        Some(Token::IntLiteral(n)) => {
            *pos += 1;
            Ok(TypedExpr::Int(*n as i64))
        }
        Some(Token::FloatLiteral(f)) => {
            *pos += 1;
            Ok(TypedExpr::Float(*f))
        }
        Some(Token::True) => {
            *pos += 1;
            Ok(TypedExpr::Bool(true))
        }
        Some(Token::False) => {
            *pos += 1;
            Ok(TypedExpr::Bool(false))
        }
        Some(Token::Nil) => {
            *pos += 1;
            Ok(TypedExpr::Nil)
        }
        Some(Token::OpenBracket) => parse_list_literal(tokens, pos),
        Some(Token::Ident(name)) => {
            let n = name.clone();
            *pos += 1;
            Ok(TypedExpr::Var {
                name: n,
                type_: ValType::I64,
            })
        }
        Some(Token::OpenParen) => {
            // Tuple or group
            *pos += 1;
            skip_whitespace(tokens, pos);
            let inner = parse_expression(tokens, pos)?;
            skip_whitespace(tokens, pos);
            expect(tokens, pos, Token::CloseParen)?;
            Ok(inner)
        }
        Some(Token::Case) => parse_case(tokens, pos),
        Some(Token::Fn) => parse_closure(tokens, pos),
        _ => Err(error_at(*pos, "expected expression")),
    }
}

fn parse_list_literal(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    // Handle list literal: [1, 2, 3] or [head, ..tail]
    *pos += 1; // skip '['
    skip_whitespace(tokens, pos);
    let mut elements = Vec::new();
    while !matches!(peek(tokens, *pos), Some(Token::CloseBracket)) {
        if *pos >= tokens.len() {
            return Err(error_at(*pos, "unexpected end of list"));
        }
        elements.push(parse_expression(tokens, pos)?);
        skip_whitespace(tokens, pos);
        if matches!(peek(tokens, *pos), Some(Token::Comma)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
        }
        // Handle spread: [head, ..tail]
        if matches!(peek(tokens, *pos), Some(Token::DotDot)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
            let tail = parse_expression(tokens, pos)?;
            // Build Cons chain from elements, append tail
            let mut expr: TypedExpr = tail;
            for elem in elements.into_iter().rev() {
                expr = TypedExpr::ListCons {
                    head: Box::new(elem),
                    tail: Box::new(expr),
                    type_: ValType::I64,
                };
            }
            skip_whitespace(tokens, pos);
            expect(tokens, pos, Token::CloseBracket)?;
            return Ok(expr);
        }
    }
    *pos += 1; // skip ']'
    // Build Cons chain, terminated by Nil
    let mut expr: TypedExpr = TypedExpr::ListNil;
    for elem in elements.into_iter().rev() {
        expr = TypedExpr::ListCons {
            head: Box::new(elem),
            tail: Box::new(expr),
            type_: ValType::I64,
        };
    }
    Ok(expr)
}

fn parse_case(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    *pos += 1; // skip 'case'
    skip_whitespace(tokens, pos);
    let scrutinee = parse_expression(tokens, pos)?;
    skip_whitespace(tokens, pos);
    expect(tokens, pos, Token::OpenBrace)?;
    skip_whitespace(tokens, pos);

    let mut cases = Vec::new();
    while !matches!(peek(tokens, *pos), Some(Token::CloseBrace)) {
        if *pos >= tokens.len() {
            break;
        }
        // Pattern -> body
        // Pattern can be: VariantName(a, b) | VariantName(a, rest..) | VariantName | _
        let variant_index = if matches!(peek(tokens, *pos), Some(Token::Ident(_))) {
            let vname = tokens[*pos].as_ident().unwrap().to_string();
            *pos += 1;
            // Map common variant names to indices
            match vname.as_str() {
                "Ok" | "Error" | "True" | "Some" => 1,
                _ => 0,
            }
        } else if matches!(peek(tokens, *pos), Some(Token::Underscore)) {
            *pos += 1;
            0 // wildcard
        } else {
            return Err(error_at(*pos, "expected pattern"));
        };

        let mut bindings = Vec::new();
        skip_whitespace(tokens, pos);
        // Optional constructor args: Pattern(a, b) or Pattern(name: a, ..)
        if matches!(peek(tokens, *pos), Some(Token::OpenParen)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
            while !matches!(peek(tokens, *pos), Some(Token::CloseParen)) {
                // Skip field label if present: label: name
                if matches!(peek(tokens, *pos), Some(Token::Ident(_))) {
                    let field = tokens[*pos].as_ident().unwrap().to_string();
                    *pos += 1;
                    skip_whitespace(tokens, pos);
                    if matches!(peek(tokens, *pos), Some(Token::Colon)) {
                        *pos += 1;
                        skip_whitespace(tokens, pos);
                        if let Some(Token::Ident(bind)) = tokens.get(*pos) {
                            bindings.push(bind.clone());
                            *pos += 1;
                        }
                    } else {
                        bindings.push(field);
                    }
                }
                skip_whitespace(tokens, pos);
                if matches!(peek(tokens, *pos), Some(Token::Comma)) {
                    *pos += 1;
                    skip_whitespace(tokens, pos);
                }
                // Skip double-dot spread: rest..
                if matches!(peek(tokens, *pos), Some(Token::DotDot)) {
                    *pos += 1;
                    break; // rest of args ignored
                }
            }
            skip_whitespace(tokens, pos);
            if matches!(peek(tokens, *pos), Some(Token::CloseParen)) {
                *pos += 1;
            }
        }

        skip_whitespace(tokens, pos);
        expect(tokens, pos, Token::Arrow)?;
        skip_whitespace(tokens, pos);
        let body = parse_expression(tokens, pos)?;

        cases.push(crate::codegen::MatchCase {
            variant_index,
            bindings,
            body: Box::new(body),
        });

        skip_whitespace(tokens, pos);
        if matches!(peek(tokens, *pos), Some(Token::Newline)) {
            *pos += 1;
            skip_whitespace(tokens, pos);
        }
    }
    expect(tokens, pos, Token::CloseBrace)?;

    Ok(TypedExpr::Match {
        scrutinee: Box::new(scrutinee),
        cases,
        type_: ValType::I64,
    })
}

fn parse_closure(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
    *pos += 1; // skip 'fn'
    let params = parse_param_list(tokens, pos)?;
    skip_whitespace(tokens, pos);
    let body = parse_block(tokens, pos)?;
    Ok(TypedExpr::Closure {
        params: params.into_iter().map(|p| (p, ValType::I64)).collect(),
        captured: vec![],
        body: Box::new(body),
        type_: ValType::I64,
        inner_func_idx: None,
    })
}

fn binop_token(tok: Option<&Token>) -> Option<crate::codegen::ast::BinOp> {
    match tok? {
        Token::Plus => Some(crate::codegen::ast::BinOp::Add),
        Token::Minus => Some(crate::codegen::ast::BinOp::Sub),
        Token::Star => Some(crate::codegen::ast::BinOp::Mul),
        Token::Slash => Some(crate::codegen::ast::BinOp::Div),
        Token::EqEq => Some(crate::codegen::ast::BinOp::Eq),
        Token::NotEq => Some(crate::codegen::ast::BinOp::Ne),
        Token::Lt => Some(crate::codegen::ast::BinOp::Lt),
        Token::Gt => Some(crate::codegen::ast::BinOp::Gt),
        Token::Le => Some(crate::codegen::ast::BinOp::Le),
        Token::Ge => Some(crate::codegen::ast::BinOp::Ge),
        _ => None,
    }
}
