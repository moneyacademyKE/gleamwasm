use crate::ir::module::*;
use crate::ir::types::*;

pub fn emit_wat(module: &Module) -> String {
    let mut out = String::new();
    out.push_str("(module\n");

    emit_types(&mut out, &module.types);
    emit_imports(&mut out, &module.imports);

    for mem in &module.memories {
        out.push_str(&format!("  (memory {}", mem.min));
        if let Some(max) = mem.max {
            out.push_str(&format!(" {max}"));
        }
        out.push_str(")\n");
        out.push_str("  (export \"memory\" (memory 0))\n");
    }

    for global in &module.globals {
        let mut_str = if global.type_.mutable { " (mut " } else { " (" };
        out.push_str(&format!("  (global{}{})", mut_str, global.type_.val_type));
        if !global.init.is_empty() {
            out.push_str("\n    ");
            for instr in &global.init {
                out.push_str(&format!("{instr} "));
            }
        }
        out.push_str(")\n");
    }

    for func in &module.functions {
        emit_function(&mut out, func);
    }

    emit_exports(&mut out, &module.exports);

    out.push_str(")\n");
    out
}

fn emit_types(out: &mut String, types: &[TypeDef]) {
    for td in types {
        match &td.kind {
            TypeDefKind::Struct(st) => {
                out.push_str("  (type");
                if let Some(ref name) = st.name {
                    out.push_str(&format!(" {name}"));
                }
                if let Some(super_) = st.supertype {
                    let super_name = types
                        .get(super_ as usize)
                        .and_then(|t| match &t.kind {
                            TypeDefKind::Struct(s) => s.name.as_deref(),
                            _ => None,
                        })
                        .unwrap_or("?");
                    out.push_str(&format!(" (sub {super_name})"));
                } else {
                    out.push_str(" (sub");
                }
                out.push_str(" (struct");
                for field in &st.fields {
                    out.push_str(" (field");
                    if let Some(ref fname) = field.name {
                        out.push_str(&format!(" {fname}"));
                    }
                    out.push_str(&format!(" {})", field.type_));
                }
                out.push_str("))\n");
            }
            TypeDefKind::Array(at) => {
                out.push_str("  (type");
                if let Some(ref name) = at.name {
                    out.push_str(&format!(" {name}"));
                }
                out.push_str(&format!(" (array {}))\n", at.field));
            }
            TypeDefKind::Func(ft) => {
                out.push_str("  (type (func");
                if !ft.params.is_empty() {
                    out.push_str(" (param");
                    for p in &ft.params {
                        out.push_str(&format!(" {p}"));
                    }
                    out.push(')');
                }
                if !ft.results.is_empty() {
                    out.push_str(" (result");
                    for r in &ft.results {
                        out.push_str(&format!(" {r}"));
                    }
                    out.push(')');
                }
                out.push_str("))\n");
            }
        }
    }
}

fn emit_imports(out: &mut String, imports: &[Import]) {
    for imp in imports {
        match &imp.desc {
            ImportDesc::Func { type_index } => {
                out.push_str(&format!(
                    "  (import \"{}\" \"{}\" (func (type {type_index})))\n",
                    imp.module, imp.name
                ));
            }
        }
    }
}

fn emit_exports(out: &mut String, exports: &[Export]) {
    for exp in exports {
        match exp.kind {
            ExportKind::Func(idx) => {
                out.push_str(&format!("  (export \"{}\" (func {idx}))\n", exp.name));
            }
        }
    }
}

fn emit_function(out: &mut String, func: &Function) {
    out.push_str("  (func");
    if let Some(ref name) = func.name {
        out.push_str(&format!(" {name}"));
    }
    out.push_str(&format!(" (type {})\n", func.type_index));

    // Separate params from vars
    let params: Vec<_> = func
        .locals
        .iter()
        .filter(|l| matches!(l.kind, LocalKind::Param(_)))
        .collect();
    let vars: Vec<_> = func
        .locals
        .iter()
        .filter(|l| matches!(l.kind, LocalKind::Var(_)))
        .collect();

    for local in &params {
        if let LocalKind::Param(t) = &local.kind {
            let name = local.name.as_deref().unwrap_or("$param");
            out.push_str(&format!("    (param {name} {t})\n"));
        }
    }

    for local in &vars {
        if let LocalKind::Var(t) = &local.kind {
            let name = local.name.as_deref().unwrap_or("$var");
            out.push_str(&format!("    (local {name} {t})\n"));
        }
    }

    // Emit body
    for instr in &func.body {
        out.push_str(&format!("    {instr}\n"));
    }

    out.push_str("  )\n");
}
