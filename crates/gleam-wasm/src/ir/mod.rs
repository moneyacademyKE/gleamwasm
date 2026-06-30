pub mod function;
pub mod module;
pub mod types;

pub use module::*;
pub use types::{
    ArrayType, Block, FieldType, HEAP_TYPE_ANY, HEAP_TYPE_DATA, HEAP_TYPE_EXTERN, HEAP_TYPE_FUNC,
    HEAP_TYPE_I31, HEAP_TYPE_NOEXTERN, HEAP_TYPE_NONE, HEAP_TYPE_STRUCT, Instr, Local, LocalKind,
    RefType, StructType, ValType,
};
