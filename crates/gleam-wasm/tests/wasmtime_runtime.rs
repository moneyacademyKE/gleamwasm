use gleam_wasm::emit::emit_wat;
use gleam_wasm::ir::module::*;
use gleam_wasm::ir::types::*;
use std::io::Write;
use std::process::Command;

fn wasmtime_available() -> bool {
    Command::new("wasmtime")
        .args(["--version"])
        .output()
        .is_ok()
}

fn run_wasm_module(wat: &str, export_name: &str) -> Option<String> {
    if !wasmtime_available() {
        return None;
    }

    let mut child = Command::new("wasmtime")
        .args(["run", "--invoke", export_name, "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(wat.as_bytes()).ok()?;
    }
    // stdin dropped

    let output = child.wait_with_output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Some(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("wasmtime stderr: {stderr}");
        None
    }
}

/// Build a WAT module that wraps `add(a, b)` into a zero-arg export `add_test`
/// so wasmtime --invoke has concrete arguments.
fn make_add_module() -> String {
    let mut module = Module::new();

    let fn_type = module.add_func_type(FuncType {
        params: vec![ValType::I64, ValType::I64],
        results: vec![ValType::I64],
    });
    let wrap_type = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I64],
    });

    module.functions.push(Function {
        name: Some("$add".into()),
        type_index: fn_type,
        locals: vec![
            Local::param("$a", ValType::I64),
            Local::param("$b", ValType::I64),
        ],
        body: vec![Instr::LocalGet(0), Instr::LocalGet(1), Instr::I64Add],
    });

    // Wrapper: calls add(10, 32)
    module.functions.push(Function {
        name: Some("$add_test".into()),
        type_index: wrap_type,
        locals: vec![],
        body: vec![Instr::I64Const(10), Instr::I64Const(32), Instr::Call(0)],
    });

    module.exports.push(Export {
        name: "add_test".into(),
        kind: ExportKind::Func(1),
    });

    emit_wat(&module)
}

fn make_factorial_module() -> String {
    let mut module = Module::new();

    let fn_type = module.add_func_type(FuncType {
        params: vec![ValType::I64],
        results: vec![ValType::I64],
    });
    let wrap_type = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I64],
    });

    module.functions.push(Function {
        name: Some("$factorial".into()),
        type_index: fn_type,
        locals: vec![Local::param("$n", ValType::I64)],
        body: vec![
            Instr::LocalGet(0),
            Instr::I64Const(1),
            Instr::I64LeS,
            Instr::If {
                then_branch: Box::new(Block::new(vec![Instr::I64Const(1)], None).label("base")),
                else_branch: Some(Box::new(
                    Block::new(
                        vec![
                            Instr::LocalGet(0),
                            Instr::LocalGet(0),
                            Instr::I64Const(1),
                            Instr::I64Sub,
                            Instr::Call(0),
                            Instr::I64Mul,
                        ],
                        None,
                    )
                    .label("recurse"),
                )),
            },
        ],
    });

    // Wrapper: factorial(5)
    module.functions.push(Function {
        name: Some("$fact_test".into()),
        type_index: wrap_type,
        locals: vec![],
        body: vec![Instr::I64Const(5), Instr::Call(0)],
    });

    module.exports.push(Export {
        name: "fact_test".into(),
        kind: ExportKind::Func(1),
    });

    emit_wat(&module)
}

fn make_answer_module() -> String {
    let mut module = Module::new();
    let fn_type = module.add_func_type(FuncType {
        params: vec![],
        results: vec![ValType::I32],
    });

    module.functions.push(Function {
        name: Some("$answer".into()),
        type_index: fn_type,
        locals: vec![],
        body: vec![Instr::I32Const(42)],
    });

    module.exports.push(Export {
        name: "answer".into(),
        kind: ExportKind::Func(0),
    });

    emit_wat(&module)
}

#[test]
fn test_wasmtime_addition() {
    let wat = make_add_module();
    println!("{wat}");

    if wasmtime_available() {
        let result = run_wasm_module(&wat, "add_test");
        assert_eq!(result.as_deref(), Some("42"), "10 + 32 should be 42");
    } else {
        println!("wasmtime not found, skipping runtime test");
    }
}

#[test]
fn test_wasmtime_factorial() {
    let wat = make_factorial_module();
    println!("{wat}");

    if wasmtime_available() {
        let result = run_wasm_module(&wat, "fact_test");
        assert_eq!(result.as_deref(), Some("120"), "5! should be 120");
    } else {
        println!("wasmtime not found, skipping runtime test");
    }
}

#[test]
fn test_wasmtime_simple_export() {
    let wat = make_answer_module();
    println!("{wat}");

    if wasmtime_available() {
        let result = run_wasm_module(&wat, "answer");
        assert_eq!(result.as_deref(), Some("42"));
    } else {
        println!("wasmtime not found, skipping runtime test");
    }
}
