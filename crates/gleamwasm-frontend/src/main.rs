//! gleamwasm-frontend — sidecar typed-AST extractor (ADR-001, Option A2).
//!
//! Usage: gleamwasm-frontend <file.gleam>
//!
//! Runs gleam-core's parser + type inference for a standalone module
//! (no imports, JavaScript target) using gleam-core's public analysis
//! API (the same recipe its own test harness uses, copied here because
//! `type_::tests` is cfg(test)-gated), then lowers the typed AST to the
//! versioned JSON schema `gleamwasm-typed-ast/v1` on stdout.
//!
//! MVP scope: functions over Int/Float/Bool/Nil/String values, simple
//! `let x = ...` bindings, arithmetic/comparison binops, direct local
//! calls, blocks. Everything else exits 1 with an explicit
//! "unsupported" message — silent miscompiles are the enemy.

use camino::Utf8PathBuf;
use gleam_core::analyse::{ModuleAnalyzerConstructor, TargetSupport};
use gleam_core::ast::{BinOp as GBinOp, TypedExpr, TypedPattern, TypedStatement};
use gleam_core::build::{Origin, Outcome, Target};
use gleam_core::config::PackageConfig;
use gleam_core::line_numbers::LineNumbers;
use gleam_core::parse::parse_module;
use gleam_core::type_::{build_prelude, Type};
use gleam_core::uid::UniqueIdGenerator;
use gleam_core::warning::{TypeWarningEmitter, VectorWarningEmitterIO, WarningEmitter};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

const SCHEMA: &str = "gleamwasm-typed-ast/v1";
/// gleam_core::type_::prelude::PRELUDE_MODULE_NAME (kept as a literal to
/// avoid depending on a re-export path; it has been "gleam" since v0.x).
const PRELUDE: &str = "gleam";

#[derive(Serialize)]
struct Envelope {
    schema: String,
    module: ModuleJson,
}

#[derive(Serialize)]
struct ModuleJson {
    functions: Vec<FnJson>,
    exports: Vec<String>,
}

#[derive(Serialize)]
struct FnJson {
    name: String,
    params: Vec<ParamJson>,
    return_type: String,
    body: ExprJson,
}

#[derive(Serialize)]
struct ParamJson {
    name: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Serialize)]
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

fn main() {
    if let Err(msg) = run() {
        eprintln!("gleamwasm-frontend error: {msg}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: gleamwasm-frontend <file.gleam>")?;
    let src = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;

    // --- gleam-core standalone inference recipe (mirrors its test harness) ---
    let ids = UniqueIdGenerator::new();
    let mut modules = im::HashMap::new();
    let emitter = WarningEmitter::new(Rc::new(VectorWarningEmitterIO::default()));
    let _ = modules.insert(PRELUDE.into(), build_prelude(&ids));

    let parsed = parse_module(Utf8PathBuf::from(path.as_str()), &src, &emitter)
        .map_err(|e| format!("parse error: {e:?}"))?;
    let mut ast = parsed.module;
    ast.name = "gleamwasm_app".into();

    let mut config = PackageConfig::default();
    config.name = "thepackage".into();

    let outcome = ModuleAnalyzerConstructor::<()> {
        target: Target::JavaScript,
        ids: &ids,
        origin: Origin::Src,
        importable_modules: &modules,
        warnings: &TypeWarningEmitter::null(),
        direct_dependencies: &HashMap::new(),
        dev_dependencies: &HashSet::new(),
        target_support: TargetSupport::NotEnforced,
        package_config: &config,
    }
    .infer_module(ast, LineNumbers::new(&src), "".into());

    let module = match outcome {
        Outcome::Ok(m) => m,
        Outcome::PartialFailure(m, _) => m,
        Outcome::TotalFailure(e) => return Err(format!("type inference failed: {:?}", e.first())),
    };

    let defs = &module.definitions;
    if !defs.imports.is_empty() {
        return Err("imports unsupported (MVP: standalone modules only)".into());
    }
    if !defs.constants.is_empty() {
        return Err("module constants unsupported (MVP)".into());
    }
    if !defs.custom_types.is_empty() {
        return Err("custom types unsupported (MVP)".into());
    }
    if !defs.type_aliases.is_empty() {
        return Err("type aliases unsupported (MVP)".into());
    }

    let mut functions = Vec::new();
    let mut exports = Vec::new();
    for f in &defs.functions {
        let name = f
            .name
            .as_ref()
            .ok_or("unnamed function definition")?
            .1
            .to_string();
        if f.publicity.is_public() {
            exports.push(name.clone());
        }
        let mut params = Vec::new();
        for arg in &f.arguments {
            let pname = arg
                .get_variable_name()
                .ok_or("discarded parameters unsupported (MVP)")?
                .to_string();
            params.push(ParamJson {
                name: pname,
                ty: val_type_name(&arg.type_)?,
            });
        }
        let return_type = val_type_name(&f.return_type)?;
        let body = lower_stmts(&f.body)?;
        functions.push(FnJson {
            name,
            params,
            return_type,
            body,
        });
    }

    let envelope = Envelope {
        schema: SCHEMA.into(),
        module: ModuleJson { functions, exports },
    };
    println!(
        "{}",
        serde_json::to_string(&envelope).map_err(|e| format!("serialize: {e}"))?
    );
    Ok(())
}

/// Map a gleam-core Type onto our ValType schema names (MVP: scalars).
/// Inference leaves resolved type variables as links; collapse first.
fn val_type_name(t: &Arc<Type>) -> Result<String, String> {
    let t = gleam_core::type_::collapse_links(t.clone());
    match t.as_ref() {
        Type::Named { name, .. } => match name.as_str() {
            "Int" => Ok("I64".into()),
            "Float" => Ok("F64".into()),
            "Bool" => Ok("I32".into()),
            other => Err(format!("unsupported type `{other}` (MVP: Int/Float/Bool)")),
        },
        other => Err(format!(
            "unsupported type shape (MVP: Int/Float/Bool apps only): {other:?}"
        )),
    }
}

/// Fold a statement list into one expression; lets wrap the tail.
fn lower_stmts(stmts: &[TypedStatement]) -> Result<ExprJson, String> {
    match stmts.split_first() {
        None => Err("empty function body".into()),
        Some((TypedStatement::Expression(e), [])) => lower_expr(e),
        Some((TypedStatement::Expression(_), _)) => {
            Err("non-tail bare expression unsupported (MVP)".into())
        }
        Some((TypedStatement::Assignment(a), rest)) => {
            let TypedPattern::Variable { name, type_, .. } = &a.pattern else {
                return Err("only simple `let x = ...` bindings supported (MVP)".into());
            };
            Ok(ExprJson::Let {
                name: name.to_string(),
                value: Box::new(lower_expr(&a.value)?),
                body: Box::new(lower_stmts(rest)?),
                ty: val_type_name(type_)?,
            })
        }
        Some((TypedStatement::Use(_), _)) => Err("`use` expressions unsupported (MVP)".into()),
        Some((TypedStatement::Assert(_), _)) => Err("`assert` unsupported (MVP)".into()),
    }
}

fn lower_expr(e: &TypedExpr) -> Result<ExprJson, String> {
    match e {
        TypedExpr::Int { int_value, .. } => {
            let n: i64 = int_value
                .to_str_radix(10)
                .parse()
                .map_err(|_| format!("integer literal out of i64 range: {int_value}"))?;
            Ok(ExprJson::Int(n))
        }
        TypedExpr::Float { value, .. } => {
            let f: f64 = value
                .parse()
                .map_err(|_| format!("bad float literal: {value}"))?;
            Ok(ExprJson::Float(f))
        }
        TypedExpr::String { value, .. } => Ok(ExprJson::StringLiteral(value.to_string())),
        TypedExpr::Var {
            name, constructor, ..
        } => {
            // Prelude constructors True/False/Nil arrive as Vars.
            match name.as_str() {
                "True" => return Ok(ExprJson::Bool(true)),
                "False" => return Ok(ExprJson::Bool(false)),
                "Nil" => return Ok(ExprJson::Nil),
                _ => {}
            }
            Ok(ExprJson::Var {
                name: name.to_string(),
                ty: val_type_name(&constructor.type_)?,
            })
        }
        TypedExpr::BinOp {
            operator,
            left,
            right,
            type_,
            ..
        } => Ok(ExprJson::BinOp {
            op: map_binop(operator)?.into(),
            left: Box::new(lower_expr(left)?),
            right: Box::new(lower_expr(right)?),
            ty: val_type_name(type_)?,
        }),
        TypedExpr::Call {
            fun,
            arguments,
            type_,
            ..
        } => {
            let TypedExpr::Var { name, .. } = fun.as_ref() else {
                return Err("only direct calls to named functions supported (MVP)".into());
            };
            let mut args = Vec::new();
            for a in arguments {
                if a.label.is_some() {
                    return Err("labelled arguments unsupported (MVP)".into());
                }
                args.push(lower_expr(&a.value)?);
            }
            Ok(ExprJson::Call {
                name: name.to_string(),
                args,
                ty: val_type_name(type_)?,
            })
        }
        TypedExpr::Block { statements, .. } => lower_stmts(statements),
        other => {
            let dump = format!("{other:?}");
            let head: String = dump.chars().take(48).collect();
            Err(format!("unsupported expression (MVP): {head}..."))
        }
    }
}

fn map_binop(op: &GBinOp) -> Result<&'static str, String> {
    Ok(match op {
        GBinOp::AddInt | GBinOp::AddFloat => "Add",
        GBinOp::SubInt | GBinOp::SubFloat => "Sub",
        GBinOp::MultInt | GBinOp::MultFloat => "Mul",
        GBinOp::DivInt | GBinOp::DivFloat => "Div",
        GBinOp::Eq => "Eq",
        GBinOp::NotEq => "Ne",
        GBinOp::LtInt | GBinOp::LtFloat => "Lt",
        GBinOp::LtEqInt | GBinOp::LtEqFloat => "Le",
        GBinOp::GtInt | GBinOp::GtFloat => "Gt",
        GBinOp::GtEqInt | GBinOp::GtEqFloat => "Ge",
        other => return Err(format!("unsupported operator (MVP): {other:?}")),
    })
}
