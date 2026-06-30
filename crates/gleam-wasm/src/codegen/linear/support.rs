    let wat = crate::emit::emit_wat(&module);
    (module, wat)
}

fn linear_type(vt: &ValType) -> ValType {
    match vt {
        ValType::I64 | ValType::I32 => ValType::I64,
        ValType::F64 => ValType::F64,
        ValType::Ref(_) => ValType::I64,
        _ => ValType::I64,
    }
}
