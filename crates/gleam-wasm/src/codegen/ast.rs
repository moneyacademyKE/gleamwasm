#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nil,
    Var {
        name: String,
        type_: crate::ir::types::ValType,
    },
    BinOp {
        op: BinOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    Call {
        name: String,
        args: Vec<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    CallClosure {
        closure: Box<TypedExpr>,
        args: Vec<TypedExpr>,
        type_: crate::ir::types::ValType,
        closure_type_index: u32,
        func_type_index: u32,
    },
    TailCall {
        name: String,
        args: Vec<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    If {
        cond: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    Let {
        name: String,
        value: Box<TypedExpr>,
        body: Box<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    Closure {
        params: Vec<(String, crate::ir::types::ValType)>,
        captured: Vec<String>,
        body: Box<TypedExpr>,
        type_: crate::ir::types::ValType,
        inner_func_idx: Option<u32>,
    },
    StructNew {
        type_index: u32,
        fields: Vec<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    StructGet {
        expr: Box<TypedExpr>,
        type_index: u32,
        field_index: u32,
        type_: crate::ir::types::ValType,
    },
    Match {
        scrutinee: Box<TypedExpr>,
        cases: Vec<MatchCase>,
        type_: crate::ir::types::ValType,
    },
    Tuple {
        elements: Vec<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    TupleGet {
        tuple: Box<TypedExpr>,
        type_index: u32,
        element_index: u32,
        type_: crate::ir::types::ValType,
    },
    StringLiteral(String),
    ListNil,
    ListCons {
        head: Box<TypedExpr>,
        tail: Box<TypedExpr>,
        type_: crate::ir::types::ValType,
    },
    Panic {
        message: String,
        type_: crate::ir::types::ValType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub variant_index: u32,
    pub bindings: Vec<String>,
    pub body: Box<TypedExpr>,
}
