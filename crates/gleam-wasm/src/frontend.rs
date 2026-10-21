//! The A2 data boundary (ADR-001): consume the `gleamwasm-typed-ast/v1`
//! JSON emitted by the `gleamwasm-frontend` sidecar and lower it into our
//! `GleamModule`/`TypedExpr` IR. gleam-core never links into this crate —
//! the boundary is data.

use crate::codegen::ast::BinOp;
use crate::codegen::{GleamFunctionDef, GleamModule, TypedExpr};
use crate::ir::types::ValType;
use serde::Deserialize;

pub const SCHEMA: &str = "gleamwasm-typed-ast/v1";

#[derive(Deserialize)]
struct Envelope {
    schema: String,
    module: ModuleJson,
}

#[derive(Deserialize)]
struct ModuleJson {
    functions: Vec<FnJson>,
    exports: Vec<String>,
}

#[derive(Deserialize)]
struct FnJson {
    name: String,
    params: Vec<ParamJson>,
    return_type: String,
    body: ExprJson,
}

#[derive(Deserialize)]
struct ParamJson {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Deserialize)]
enum ExprJson {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nil,
    StringLiteral(String),
    Var {
        name: String,
        #[serde(rename = "type")]
        ty: String,
    },
    BinOp {
        op: String,
        left: Box<ExprJson>,
        right: Box<ExprJson>,
        #[serde(rename = "type")]
        ty: String,
    },
    Call {
        name: String,
        args: Vec<ExprJson>,
        #[serde(rename = "type")]
        ty: String,
    },
    Let {
        name: String,
        value: Box<ExprJson>,
        body: Box<ExprJson>,
        #[serde(rename = "type")]
        ty: String,
    },
}

/// Parse + lower sidecar JSON into our IR.
pub fn module_from_json(json: &str) -> Result<GleamModule, String> {
    let envelope: Envelope =
        serde_json::from_str(json).map_err(|e| format!("frontend JSON parse error: {e}"))?;
    if envelope.schema != SCHEMA {
        return Err(format!(
            "unsupported frontend schema {:?} (expected {SCHEMA:?})",
            envelope.schema
        ));
    }
    let mut functions = Vec::new();
    for f in envelope.module.functions {
        let mut params = Vec::new();
        for p in f.params {
            params.push((p.name, val_type(&p.ty)?));
        }
        functions.push(GleamFunctionDef {
            name: f.name,
            params,
            return_type: val_type(&f.return_type)?,
            body: lower(f.body)?,
        });
    }
    Ok(GleamModule {
        functions,
        exports: envelope.module.exports,
        imports: vec![],
        adt_types: vec![],
    })
}

fn val_type(s: &str) -> Result<ValType, String> {
    Ok(match s {
        "I32" => ValType::I32,
        "I64" => ValType::I64,
        "F32" => ValType::F32,
        "F64" => ValType::F64,
        other => return Err(format!("unknown ValType in frontend JSON: {other:?}")),
    })
}

fn binop(s: &str) -> Result<BinOp, String> {
    Ok(match s {
        "Add" => BinOp::Add,
        "Sub" => BinOp::Sub,
        "Mul" => BinOp::Mul,
        "Div" => BinOp::Div,
        "Eq" => BinOp::Eq,
        "Ne" => BinOp::Ne,
        "Lt" => BinOp::Lt,
        "Gt" => BinOp::Gt,
        "Le" => BinOp::Le,
        "Ge" => BinOp::Ge,
        other => return Err(format!("unknown binop in frontend JSON: {other:?}")),
    })
}

fn lower(e: ExprJson) -> Result<TypedExpr, String> {
    Ok(match e {
        ExprJson::Int(n) => TypedExpr::Int(n),
        ExprJson::Float(f) => TypedExpr::Float(f),
        ExprJson::Bool(b) => TypedExpr::Bool(b),
        ExprJson::Nil => TypedExpr::Nil,
        ExprJson::StringLiteral(s) => TypedExpr::StringLiteral(s),
        ExprJson::Var { name, ty } => TypedExpr::Var {
            name,
            type_: val_type(&ty)?,
        },
        ExprJson::BinOp {
            op,
            left,
            right,
            ty,
        } => TypedExpr::BinOp {
            op: binop(&op)?,
            left: Box::new(lower(*left)?),
            right: Box::new(lower(*right)?),
            type_: val_type(&ty)?,
        },
        ExprJson::Call { name, args, ty } => {
            let mut lowered = Vec::new();
            for a in args {
                lowered.push(lower(a)?);
            }
            TypedExpr::Call {
                name,
                args: lowered,
                type_: val_type(&ty)?,
            }
        }
        ExprJson::Let {
            name,
            value,
            body,
            ty,
        } => TypedExpr::Let {
            name,
            value: Box::new(lower(*value)?),
            body: Box::new(lower(*body)?),
            type_: val_type(&ty)?,
        },
    })
}

/// Locate and run the sidecar on a .gleam file; return its JSON stdout.
///
/// Discovery order: `GLEAMWASM_FRONTEND` env override, then the sibling
/// of the current executable (workspace `target/<profile>/` layout), then
/// bare `gleamwasm-frontend` on PATH.
pub fn run_sidecar(input: &str) -> Result<String, String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("GLEAMWASM_FRONTEND") {
        candidates.push(p.into());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("gleamwasm-frontend"));
        }
    }
    candidates.push("gleamwasm-frontend".into());

    let mut tried = Vec::new();
    for cand in &candidates {
        tried.push(cand.display().to_string());
        let out = match std::process::Command::new(cand).arg(input).output() {
            Ok(o) => o,
            Err(_) => continue, // not found / not executable — try next
        };
        if out.status.success() {
            return String::from_utf8(out.stdout)
                .map_err(|e| format!("sidecar stdout not utf-8: {e}"));
        }
        return Err(format!(
            "frontend sidecar failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Err(format!(
        "gleamwasm-frontend sidecar not found (tried: {}). Build it with \
         `cargo build -p gleamwasm-frontend`, or point GLEAMWASM_FRONTEND at \
         the binary, or pass --frontend builtin for the legacy I64 subset \
         parser. See docs/adr/ADR-001-frontend.md.",
        tried.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"{"schema":"gleamwasm-typed-ast/v1","module":{"functions":[{"name":"main","params":[],"return_type":"I64","body":{"Int":42}}],"exports":["main"]}}"#;

    #[test]
    fn lowers_answer_module() {
        let m = module_from_json(ANSWER).unwrap();
        assert_eq!(m.exports, vec!["main".to_string()]);
        assert_eq!(m.functions.len(), 1);
        assert_eq!(m.functions[0].return_type, ValType::I64);
        assert!(matches!(m.functions[0].body, TypedExpr::Int(42)));
    }

    #[test]
    fn rejects_wrong_schema() {
        let json = r#"{"schema":"other/v9","module":{"functions":[],"exports":[]}}"#;
        assert!(module_from_json(json).is_err());
    }

    #[test]
    fn lowers_let_binop_var() {
        let json = r#"{"schema":"gleamwasm-typed-ast/v1","module":{"functions":[{"name":"main","params":[],"return_type":"I64","body":{"Let":{"name":"a","value":{"Int":1},"body":{"BinOp":{"op":"Add","left":{"Var":{"name":"a","type":"I64"}},"right":{"Int":2},"type":"I64"}},"type":"I64"}}}],"exports":["main"]}}"#;
        let m = module_from_json(json).unwrap();
        assert!(matches!(m.functions[0].body, TypedExpr::Let { .. }));
    }

    #[test]
    fn rejects_unknown_valtype() {
        let json = r#"{"schema":"gleamwasm-typed-ast/v1","module":{"functions":[{"name":"main","params":[],"return_type":"V128","body":{"Int":1}}],"exports":[]}}"#;
        assert!(module_from_json(json).is_err());
    }
}
