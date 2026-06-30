use crate::codegen::{GleamFunctionDef, GleamModule, TypedExpr};
use crate::ir::types::ValType;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

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
                let _ = expect_ident(&tokens, &mut pos); // type name, ignore
            }

            skip_whitespace(&tokens, &mut pos);
            let body = parse_block(&tokens, &mut pos)?;

            functions.push(GleamFunctionDef {
                name: name.clone(),
                params: params
                    .into_iter()
                    .map(|p| (p, ValType::I64))
                    .collect(),
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
            while pos < tokens.len()
                && !matches!(tokens[pos], Token::Newline)
            {
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

fn parse_param_list(tokens: &[Token], pos: &mut usize) -> Result<Vec<String>, ParseError> {
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
                if matches!(peek(tokens, *pos), Some(Token::Ident(_))) {
                    *pos += 1; // skip type name
                }
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

fn parse_block(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
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

    // Fold statements into a Let chain, last expression is the result
    if statements.is_empty() {
        Ok(TypedExpr::Nil)
    } else {
        let mut body = statements.pop().unwrap();
        for stmt in statements.into_iter().rev() {
            body = TypedExpr::Let {
                name: "_".into(),
                value: Box::new(stmt),
                body: Box::new(body),
                type_: ValType::I64,
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

fn do_let(
    tokens: &[Token],
    pos: &mut usize,
    name: &str,
) -> Result<TypedExpr, ParseError> {
    let binding_name = if name == "_" { "_".to_string() } else { name.to_string() };
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
    // After = expr, look ahead for newline + continuation
    skip_whitespace(tokens, pos);
    let body = if *pos < tokens.len() && !matches!(tokens[*pos], Token::CloseBrace) {
        parse_expression(tokens, pos)?
    } else {
        TypedExpr::Nil
    };

    Ok(TypedExpr::Let {
        name: binding_name,
        value: Box::new(value),
        body: Box::new(body),
        type_: ValType::I64,
    })
}

fn parse_expression(tokens: &[Token], pos: &mut usize) -> Result<TypedExpr, ParseError> {
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
        Some(Token::Ident(name)) => {
            let n = name.clone();
            *pos += 1;
            // Check for list construction [head, ..tail]
            skip_whitespace(tokens, pos);
            if matches!(peek(tokens, *pos), Some(Token::OpenBracket)) {
                // List literal: [1, 2, 3]
                return parse_list_literal(tokens, pos, n);
            }
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
        Some(Token::Case) => {
            parse_case(tokens, pos)
        }
        Some(Token::Fn) => {
            parse_closure(tokens, pos)
        }
        _ => Err(error_at(*pos, "expected expression")),
    }
}

fn parse_list_literal(
    tokens: &[Token],
    pos: &mut usize,
    _first: String,
) -> Result<TypedExpr, ParseError> {
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

fn tokenize(source: &str) -> Vec<Token> {
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
            '(' => { tokens.push(Token::OpenParen); i += 1; }
            ')' => { tokens.push(Token::CloseParen); i += 1; }
            '{' => { tokens.push(Token::OpenBrace); i += 1; }
            '}' => { tokens.push(Token::CloseBrace); i += 1; }
            '[' => { tokens.push(Token::OpenBracket); i += 1; }
            ']' => { tokens.push(Token::CloseBracket); i += 1; }
            ':' => { tokens.push(Token::Colon); i += 1; }
            ',' => { tokens.push(Token::Comma); i += 1; }
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
            '+' => { tokens.push(Token::Plus); i += 1; }
            '-' => {
                if i + 1 < chars.len() && chars[i + 1] == '>' {
                    tokens.push(Token::Arrow);
                    i += 2;
                } else {
                    tokens.push(Token::Minus);
                    i += 1;
                }
            }
            '*' => { tokens.push(Token::Star); i += 1; }
            '/' => { tokens.push(Token::Slash); i += 1; }
            '%' => { tokens.push(Token::Percent); i += 1; }
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
            c if c.is_alphabetic() || c == '_' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() => {
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
            _ => { i += 1; }
        }
    }
    tokens
}

fn read_identifier(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    while i < chars.len()
        && (chars[i].is_alphanumeric()
            || chars[i] == '_'
            || chars[i] == '?'
            || chars[i] == '!')
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
            || (chars[i] == '.' && !has_dot
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

#[derive(Debug, Clone, PartialEq)]
enum Token {
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
    fn as_ident(&self) -> Option<&String> {
        match self {
            Token::Ident(s) => Some(s),
            _ => None,
        }
    }
}
