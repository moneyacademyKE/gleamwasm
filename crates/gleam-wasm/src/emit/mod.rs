pub mod wasm;
pub mod wat;

pub use wasm::{emit_wasm, emit_wasm_to, encode_manual};
pub use wat::emit_wat;
