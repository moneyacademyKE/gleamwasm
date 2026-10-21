pub mod wasm;
pub mod wat;

pub use wasm::{emit_wasm, emit_wasm_to};
pub use wat::emit_wat;
