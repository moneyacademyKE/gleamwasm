use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    I32Const(i32),
    I64Const(i64),
    F64Const(f64),
    RefNull(ValType),
    RefI31,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32RemS,
    I32Eq,
    I32Ne,
    I32LtS,
    I32GtS,
    I32LeS,
    I32GeS,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64RemS,
    I64Eq,
    I64Ne,
    I64LtS,
    I64GtS,
    I64LeS,
    I64GeS,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    StructNew(u32),
    StructGet {
        type_index: u32,
        field_index: u32,
    },
    StructSet {
        type_index: u32,
        field_index: u32,
    },
    ArrayNew(u32),
    ArrayGet(u32),
    ArraySet(u32),
    ArrayLen,
    RefTest(ValType),
    RefCast(ValType),
    BrOnCast {
        label: u32,
        src: ValType,
        dst: ValType,
    },
    BrOnCastFail {
        label: u32,
        src: ValType,
        dst: ValType,
    },
    Block(Box<Block>),
    Loop(Box<Block>),
    If {
        then_branch: Box<Block>,
        else_branch: Option<Box<Block>>,
    },
    Br(u32),
    BrIf(u32),
    BrTable {
        branches: Vec<u32>,
        default: u32,
    },
    Return,
    Unreachable,
    Call(u32),
    CallRef(u32),
    ReturnCall(u32),
    ReturnCallRef(u32),
    RefFunc(u32),
    Throw(u32),
    ExternConvertAny,
    AnyConvertExtern,
    Drop,
    // Stack / Select
    Select,
    // Global access
    GlobalGet(u32),
    GlobalSet(u32),
    // Linear memory operations
    I64ExtendI32U,
    I32WrapI64,
    I64Shl,
    I64ShrU,
    I64Or,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Load {
        offset: u32,
        align: u32,
    },
    I32Load8U {
        offset: u32,
        align: u32,
    },
    I32Store {
        offset: u32,
        align: u32,
    },
    I32Store8 {
        offset: u32,
        align: u32,
    },
    I64Load {
        offset: u32,
        align: u32,
    },
    I64Store {
        offset: u32,
        align: u32,
    },
    F64Load {
        offset: u32,
        align: u32,
    },
    F64Store {
        offset: u32,
        align: u32,
    },
    MemoryGrow,
    MemorySize,
    // Float conversion/truncation
    F64ConvertSI32,
    F64Trunc,
    I32TruncSatF64S,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub label: Option<String>,
    pub instrs: Vec<Instr>,
    pub result: Option<Box<ValType>>,
}

impl Block {
    pub fn new(instrs: Vec<Instr>, result: Option<ValType>) -> Self {
        Block {
            label: None,
            instrs,
            result: result.map(Box::new),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub name: Option<String>,
    pub kind: LocalKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LocalKind {
    Param(ValType),
    Var(ValType),
}

impl LocalKind {
    pub fn val_type(&self) -> ValType {
        match self {
            LocalKind::Param(t) | LocalKind::Var(t) => t.clone(),
        }
    }

    pub fn is_param(&self) -> bool {
        matches!(self, LocalKind::Param(_))
    }

    pub fn is_var(&self) -> bool {
        matches!(self, LocalKind::Var(_))
    }
}

impl Local {
    pub fn param(name: impl Into<String>, type_: ValType) -> Self {
        Local {
            name: Some(name.into()),
            kind: LocalKind::Param(type_),
        }
    }

    pub fn var(name: impl Into<String>, type_: ValType) -> Self {
        Local {
            name: Some(name.into()),
            kind: LocalKind::Var(type_),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    Ref(RefType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefType {
    Ref(HeapType),
    RefNull(HeapType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeapType {
    Abstract(AbstractHeapType),
    TypeIndex(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbstractHeapType {
    Any,
    Eq,
    I31,
    Struct,
    Array,
    None_,
    Func,
    Extern,
    NoExtern,
    Exn,
}

pub const HEAP_TYPE_ANY: HeapType = HeapType::Abstract(AbstractHeapType::Any);
pub const HEAP_TYPE_EQ: HeapType = HeapType::Abstract(AbstractHeapType::Eq);
pub const HEAP_TYPE_I31: HeapType = HeapType::Abstract(AbstractHeapType::I31);
pub const HEAP_TYPE_STRUCT: HeapType = HeapType::Abstract(AbstractHeapType::Struct);
pub const HEAP_TYPE_ARRAY: HeapType = HeapType::Abstract(AbstractHeapType::Array);
pub const HEAP_TYPE_NONE: HeapType = HeapType::Abstract(AbstractHeapType::None_);
pub const HEAP_TYPE_FUNC: HeapType = HeapType::Abstract(AbstractHeapType::Func);
pub const HEAP_TYPE_EXTERN: HeapType = HeapType::Abstract(AbstractHeapType::Extern);
pub const HEAP_TYPE_NOEXTERN: HeapType = HeapType::Abstract(AbstractHeapType::NoExtern);
pub const HEAP_TYPE_DATA: HeapType = HEAP_TYPE_STRUCT;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    pub name: Option<String>,
    pub supertype: Option<u32>,
    pub fields: Vec<FieldType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldType {
    pub name: Option<String>,
    pub type_: ValType,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayType {
    pub name: Option<String>,
    pub field: ValType,
    pub mutable: bool,
}

impl fmt::Display for ValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValType::I32 => write!(f, "i32"),
            ValType::I64 => write!(f, "i64"),
            ValType::F32 => write!(f, "f32"),
            ValType::F64 => write!(f, "f64"),
            ValType::Ref(r) => write!(f, "{r}"),
        }
    }
}

impl fmt::Display for RefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefType::Ref(h) => write!(f, "(ref {h})"),
            RefType::RefNull(h) => write!(f, "(ref null {h})"),
        }
    }
}

impl fmt::Display for HeapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeapType::Abstract(a) => write!(f, "{a}"),
            HeapType::TypeIndex(i) => write!(f, "${i}"),
        }
    }
}

impl fmt::Display for AbstractHeapType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AbstractHeapType::Any => f.write_str("any"),
            AbstractHeapType::Eq => f.write_str("eq"),
            AbstractHeapType::I31 => f.write_str("i31"),
            AbstractHeapType::Struct => f.write_str("struct"),
            AbstractHeapType::Array => f.write_str("array"),
            AbstractHeapType::None_ => f.write_str("none"),
            AbstractHeapType::Func => f.write_str("func"),
            AbstractHeapType::Extern => f.write_str("extern"),
            AbstractHeapType::NoExtern => f.write_str("noextern"),
            AbstractHeapType::Exn => f.write_str("exn"),
        }
    }
}
