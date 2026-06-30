use super::types::{ArrayType, Instr, Local, StructType, ValType};

pub type Expr = Vec<Instr>;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub types: Vec<TypeDef>,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub exports: Vec<Export>,
    pub memories: Vec<Memory>,
    pub globals: Vec<Global>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDef {
    pub index: u32,
    pub kind: TypeDefKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefKind {
    Struct(StructType),
    Array(super::types::ArrayType),
    Func(FuncType),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub desc: ImportDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportDesc {
    Func { type_index: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Option<String>,
    pub type_index: u32,
    pub locals: Vec<Local>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    Func(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    pub type_: GlobalType,
    pub init: Vec<Instr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalType {
    pub val_type: ValType,
    pub mutable: bool,
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

impl Module {
    pub fn new() -> Self {
        Module {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            exports: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn add_struct_type(&mut self, st: StructType) -> u32 {
        let index = self.types.len() as u32;
        self.types.push(TypeDef {
            index,
            kind: TypeDefKind::Struct(st),
        });
        index
    }

    pub fn add_array_type(&mut self, at: ArrayType) -> u32 {
        let index = self.types.len() as u32;
        self.types.push(TypeDef {
            index,
            kind: TypeDefKind::Array(at),
        });
        index
    }

    pub fn add_func_type(&mut self, ft: FuncType) -> u32 {
        let index = self.types.len() as u32;
        self.types.push(TypeDef {
            index,
            kind: TypeDefKind::Func(ft),
        });
        index
    }
}
