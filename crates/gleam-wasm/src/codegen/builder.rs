use crate::ir::types::{Block as IrBlock, Instr, ValType};

pub struct FunctionBuilder {
    locals: Vec<ValType>,
    body: Vec<Instr>,
}

impl Default for FunctionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionBuilder {
    pub fn new() -> Self {
        FunctionBuilder {
            locals: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn add_local(&mut self, type_: ValType) -> u32 {
        let index = self.locals.len() as u32;
        self.locals.push(type_);
        index
    }

    pub fn push(&mut self, instr: Instr) {
        self.body.push(instr);
    }

    pub fn local_get(&mut self, index: u32) {
        self.push(Instr::LocalGet(index));
    }

    pub fn local_set(&mut self, index: u32) {
        self.push(Instr::LocalSet(index));
    }

    pub fn i64_const(&mut self, v: i64) {
        self.push(Instr::I64Const(v));
    }

    pub fn f64_const(&mut self, v: f64) {
        self.push(Instr::F64Const(v));
    }

    pub fn i32_const(&mut self, v: i32) {
        self.push(Instr::I32Const(v));
    }

    pub fn i64_add(&mut self) {
        self.push(Instr::I64Add);
    }

    pub fn i64_mul(&mut self) {
        self.push(Instr::I64Mul);
    }

    pub fn i64_sub(&mut self) {
        self.push(Instr::I64Sub);
    }

    pub fn i64_div_s(&mut self) {
        self.push(Instr::I64DivS);
    }

    pub fn f64_add(&mut self) {
        self.push(Instr::F64Add);
    }

    pub fn f64_mul(&mut self) {
        self.push(Instr::F64Mul);
    }

    pub fn f64_sub(&mut self) {
        self.push(Instr::F64Sub);
    }

    pub fn f64_div(&mut self) {
        self.push(Instr::F64Div);
    }

    pub fn return_call(&mut self, func_index: u32) {
        self.push(Instr::ReturnCall(func_index));
    }

    pub fn call(&mut self, func_index: u32) {
        self.push(Instr::Call(func_index));
    }

    pub fn call_ref(&mut self, type_index: u32) {
        self.push(Instr::CallRef(type_index));
    }

    pub fn ref_func(&mut self, func_index: u32) {
        self.push(Instr::RefFunc(func_index));
    }

    pub fn throw(&mut self, tag_index: u32) {
        self.push(Instr::Throw(tag_index));
    }

    pub fn struct_set(&mut self, type_index: u32, field_index: u32) {
        self.push(Instr::StructSet {
            type_index,
            field_index,
        });
    }

    pub fn return_(&mut self) {
        self.push(Instr::Return);
    }

    pub fn drop_instr(&mut self) {
        self.push(Instr::Drop);
    }

    pub fn struct_new(&mut self, type_index: u32) {
        self.push(Instr::StructNew(type_index));
    }

    pub fn struct_get(&mut self, type_index: u32, field_index: u32) {
        self.push(Instr::StructGet {
            type_index,
            field_index,
        });
    }

    pub fn block(&mut self, label: impl Into<String>, body: Vec<Instr>) {
        let blk = IrBlock::new(body, None).label(label);
        self.push(Instr::Block(Box::new(blk)));
    }

    pub fn typed_block(&mut self, label: impl Into<String>, body: Vec<Instr>, result: ValType) {
        let blk = IrBlock::new(body, Some(result)).label(label);
        self.push(Instr::Block(Box::new(blk)));
    }

    pub fn br(&mut self, label: u32) {
        self.push(Instr::Br(label));
    }

    pub fn br_if(&mut self, label: u32) {
        self.push(Instr::BrIf(label));
    }

    pub fn br_table(&mut self, branches: Vec<u32>, default: u32) {
        self.push(Instr::BrTable { branches, default });
    }

    pub fn ref_null(&mut self, type_: ValType) {
        self.push(Instr::RefNull(type_));
    }

    pub fn ref_i31(&mut self) {
        self.push(Instr::RefI31);
    }

    pub fn into_body(self) -> Vec<Instr> {
        self.body
    }
}
