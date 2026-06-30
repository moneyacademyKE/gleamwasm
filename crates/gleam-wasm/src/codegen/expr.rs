use crate::ir::types::{HEAP_TYPE_ANY, HEAP_TYPE_NONE, HEAP_TYPE_STRUCT, Instr, RefType, ValType};

use super::ast::{BinOp, MatchCase, TypedExpr};
use super::builder::FunctionBuilder;
use super::types::TypeMapper;

pub fn compile_expr(mapper: &mut TypeMapper, expr: &TypedExpr) -> Vec<Instr> {
    let mut builder = FunctionBuilder::new();
    compile_recursive(&mut builder, mapper, expr);
    builder.into_body()
}

fn compile_recursive(builder: &mut FunctionBuilder, mapper: &mut TypeMapper, expr: &TypedExpr) {
    match expr {
        TypedExpr::Int(i) => {
            builder.i64_const(*i);
        }
        TypedExpr::Float(f) => {
            builder.f64_const(*f);
        }
        TypedExpr::Bool(b) => {
            builder.i32_const(if *b { 1 } else { 0 });
        }
        TypedExpr::Nil => {
            builder.ref_null(ValType::Ref(crate::ir::types::RefType::RefNull(
                crate::ir::types::HEAP_TYPE_NONE,
            )));
        }
        TypedExpr::Var { name, .. } => {
            let index = mapper.get_local_index(name).expect("local not found");
            builder.local_get(index);
        }
        TypedExpr::BinOp {
            op,
            left,
            right,
            type_,
        } => {
            compile_recursive(builder, mapper, left);
            compile_recursive(builder, mapper, right);
            emit_binop(builder, op, type_);
        }
        TypedExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            compile_recursive(builder, mapper, cond);
            let mut then_body = FunctionBuilder::new();
            compile_recursive(&mut then_body, mapper, then_branch);
            let then_block =
                crate::ir::types::Block::new(then_body.into_body(), None).label("then");
            let else_block = if let TypedExpr::Nil = **else_branch {
                None
            } else {
                let mut else_body = FunctionBuilder::new();
                compile_recursive(&mut else_body, mapper, else_branch);
                let b = crate::ir::types::Block::new(else_body.into_body(), None).label("else");
                Some(Box::new(b))
            };
            builder.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: else_block,
            });
        }
        TypedExpr::Let {
            name,
            value,
            body,
            type_,
        } => {
            compile_recursive(builder, mapper, value);
            let local_idx = builder.add_local(type_.clone());
            mapper.register_local(name.clone(), local_idx);
            builder.local_set(local_idx);
            compile_recursive(builder, mapper, body);
        }
        TypedExpr::Call { name, args, .. } => {
            for arg in args {
                compile_recursive(builder, mapper, arg);
            }
            let func_idx = mapper.get_function_index(name).expect("function not found");
            builder.call(func_idx);
        }
        TypedExpr::TailCall { name, args, .. } => {
            for arg in args {
                compile_recursive(builder, mapper, arg);
            }
            let func_idx = mapper.get_function_index(name).expect("function not found");
            builder.return_call(func_idx);
        }
        TypedExpr::Closure {
            params,
            captured: _,
            body: _,
            type_: _,
            inner_func_idx,
        } => {
            let param_types: Vec<ValType> = params.iter().map(|(_, t)| t.clone()).collect();
            let (closure_type_idx, _func_type_idx) =
                mapper.register_closure(param_types, ValType::I64);

            builder.struct_new(closure_type_idx);
            if let Some(inner_func) = inner_func_idx {
                builder.ref_func(*inner_func);
                builder.struct_set(closure_type_idx, 0); // $code
                builder.ref_null(ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)));
                builder.struct_set(closure_type_idx, 1); // $env
            }
        }
        TypedExpr::CallClosure {
            closure,
            args,
            closure_type_index,
            func_type_index,
            ..
        } => {
            for arg in args {
                compile_recursive(builder, mapper, arg);
            }
            compile_recursive(builder, mapper, closure);
            builder.struct_get(*closure_type_index, 0); // get $code from $Closure
            builder.call_ref(*func_type_index);
        }
        TypedExpr::StructNew {
            type_index, fields, ..
        } => {
            for field in fields {
                compile_recursive(builder, mapper, field);
            }
            builder.struct_new(*type_index);
        }
        TypedExpr::StructGet {
            expr,
            type_index,
            field_index,
            ..
        } => {
            compile_recursive(builder, mapper, expr);
            builder.struct_get(*type_index, *field_index);
        }
        TypedExpr::Match {
            scrutinee,
            cases,
            type_,
        } => {
            compile_match(builder, mapper, scrutinee, cases, type_);
        }
        TypedExpr::Tuple { elements, .. } => {
            // Collect element types for tuple registration
            let types: Vec<ValType> = elements
                .iter()
                .map(|_e| {
                    // Infer type from expression — simplified to anyref for now
                    ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))
                })
                .collect();
            let arity = elements.len();
            let tuple_type_idx = mapper.get_or_register_tuple(arity, types);
            for elem in elements {
                compile_recursive(builder, mapper, elem);
            }
            builder.struct_new(tuple_type_idx);
        }
        TypedExpr::TupleGet {
            tuple,
            type_index,
            element_index,
            ..
        } => {
            compile_recursive(builder, mapper, tuple);
            builder.struct_get(*type_index, *element_index);
        }
        TypedExpr::StringLiteral(_s) => {
            builder.ref_null(ValType::Ref(RefType::RefNull(
                crate::ir::types::HEAP_TYPE_EXTERN,
            )));
        }
        TypedExpr::ListNil => {
            builder.ref_null(ValType::Ref(RefType::RefNull(HEAP_TYPE_NONE)));
        }
        TypedExpr::ListCons { head, tail, .. } => {
            // Register List ADT if not already registered
            let list_idx = mapper.get_type_index("List");
            let cons_idx = match list_idx {
                Some(base) => {
                    // Find the Cons subtype (registered as base+1)
                    base + 1
                }
                None => {
                    let base = mapper.register_adt(
                        "List",
                        &[
                            (
                                "Cons",
                                vec![
                                    ("head", ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY))),
                                    ("tail", ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT))),
                                ],
                            ),
                            ("Nil", vec![]),
                        ],
                    );
                    base + 1
                }
            };

            compile_recursive(builder, mapper, head);
            compile_recursive(builder, mapper, tail);
            builder.struct_new(cons_idx);
        }
        TypedExpr::Panic { type_: _type_, .. } => {
            let tag_idx = match mapper.get_type_index("$ExnTag") {
                Some(i) => i,
                None => mapper.register_adt("$ExnTag", &[("Exn", vec![])]),
            };
            builder.ref_null(ValType::Ref(RefType::RefNull(HEAP_TYPE_ANY)));
            builder.throw(tag_idx);
        }
    }
}

fn compile_match(
    builder: &mut FunctionBuilder,
    mapper: &mut TypeMapper,
    scrutinee: &TypedExpr,
    cases: &[MatchCase],
    _result_type: &ValType,
) {
    use crate::ir::types::{Block, Instr, ValType};

    compile_recursive(builder, mapper, scrutinee);

    let scrutinee_type = ValType::Ref(RefType::RefNull(HEAP_TYPE_STRUCT));
    let scrutinee_local = builder.add_local(scrutinee_type.clone());
    builder.local_set(scrutinee_local);
    let num_cases = cases.len();

    // Build dispatch + all case bodies into one flat list, then wrap in a block.
    // After each matched case body, `br 0` exits the block.
    let mut match_body: Vec<Instr> = Vec::new();

    for (i, case) in cases.iter().enumerate() {
        let is_last = i == num_cases - 1;

        let cast_dst = ValType::Ref(RefType::RefNull(crate::ir::types::HeapType::TypeIndex(
            case.variant_index,
        )));

        // ref.test scrutinee is variant type
        match_body.push(Instr::LocalGet(scrutinee_local));
        match_body.push(Instr::RefTest(cast_dst));

        let mut then_body: Vec<Instr> = Vec::new();
        then_body.push(Instr::LocalGet(scrutinee_local));
        let cast_ref = ValType::Ref(RefType::Ref(crate::ir::types::HeapType::TypeIndex(
            case.variant_index,
        )));
        then_body.push(Instr::RefCast(cast_ref));

        let mut case_local_builder = FunctionBuilder::new();
        for (j, binding) in case.bindings.iter().enumerate() {
            case_local_builder.struct_get(case.variant_index, j as u32);
            let field_type = mapper
                .get_variant_field_type(case.variant_index, j as u32)
                .cloned()
                .unwrap_or(ValType::I64);
            let local_idx = case_local_builder.add_local(field_type);
            mapper.register_local(binding.clone(), local_idx);
            case_local_builder.local_set(local_idx);
        }
        then_body.extend(case_local_builder.into_body());

        let mut case_body_builder = FunctionBuilder::new();
        compile_recursive(&mut case_body_builder, mapper, &case.body);
        then_body.extend(case_body_builder.into_body());

        // After case body executes, exit the match block
        // br 2: skip then_block (depth 0), skip the If (depth 1), land at match_exit (depth 2)
        then_body.push(Instr::Br(2));

        let then_block = Block::new(then_body, None).label(format!("case_{}", case.variant_index));

        if is_last {
            let else_block = Block::new(vec![Instr::Unreachable], None).label("no_match");
            match_body.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: Some(Box::new(else_block)),
            });
        } else {
            match_body.push(Instr::If {
                then_branch: Box::new(then_block),
                else_branch: None,
            });
        }
    }

    // Wrap entire match dispatch in a labeled block so br 0 exits correctly
    let match_block = Block::new(match_body, None).label("match_exit");
    builder.push(Instr::Block(Box::new(match_block)));
}

fn emit_binop(builder: &mut FunctionBuilder, op: &BinOp, type_: &ValType) {
    match type_ {
        ValType::I64 => match op {
            BinOp::Add => builder.i64_add(),
            BinOp::Sub => builder.i64_sub(),
            BinOp::Mul => builder.i64_mul(),
            BinOp::Div => builder.i64_div_s(),
            BinOp::Eq => builder.push(Instr::I64Eq),
            BinOp::Ne => builder.push(Instr::I64Ne),
            BinOp::Lt => builder.push(Instr::I64LtS),
            BinOp::Gt => builder.push(Instr::I64GtS),
            BinOp::Le => builder.push(Instr::I64LeS),
            BinOp::Ge => builder.push(Instr::I64GeS),
        },
        ValType::F64 => match op {
            BinOp::Add => builder.f64_add(),
            BinOp::Sub => builder.f64_sub(),
            BinOp::Mul => builder.f64_mul(),
            BinOp::Div => builder.f64_div(),
            BinOp::Eq => builder.push(Instr::F64Eq),
            BinOp::Ne => builder.push(Instr::F64Ne),
            BinOp::Lt => builder.push(Instr::F64Lt),
            BinOp::Gt => builder.push(Instr::F64Gt),
            BinOp::Le => builder.push(Instr::F64Le),
            BinOp::Ge => builder.push(Instr::F64Ge),
        },
        _ => {
            builder.push(Instr::Unreachable);
        }
    }
}
