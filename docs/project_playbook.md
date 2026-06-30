# Project Playbook & Learnings Log

This document serves as the developer playbook and learnings log for the Direct Gleam-to-Wasm GC Compiler project.

---

## 1. Project Playbook

### Developer Guidelines:
1. **File Size Limit:** Keep all compiler files under 250 Lines of Code (LOC) to enforce high cohesion and simple components.
2. **Red/Green TDD:** Every compiler feature (e.g. adding a new primitive mapping) must be preceded by a test case asserting the output of that feature in a Wasm engine.
3. **Rust Compiler Design:** Place codegen logic in `compiler-core/src/wasm.rs` (or a dedicated subdirectory) and avoid mixing logic with Erlang or JavaScript targets to keep coupling minimal.

### Build and Test Command Reference:
* To compile Gleam to Wasm:
  ```bash
  gleam build --target wasm-web
  ```
* To run integration test suite:
  ```bash
  cargo test -p gleam-core --test wasm_compiler
  ```

---

## 2. Learnings & Patterns

### WebAssembly GC Design Patterns:
* **Structural Subtyping:** Wasm GC’s native subtyping `(sub)` is highly effective for representing polymorphic types and tagged unions. Relying on `br_on_cast` yields significantly faster pattern matching than implementing custom ID tags.
* **Immediate vs. Allocated References:** Use `i31ref` for integers and boolean values when coercing to `anyref`. Tagging a small integer avoids heap allocation overhead completely.
* **JS String Builtins:** Using `externref` combined with `wasm:js-string` imports is the optimal way to handle strings on the web. It avoids the double-copy bottleneck of decoding text from linear memory.
* **Dual-Target Selection:** Emitting Wasm GC structs when possible avoids runtime allocation overhead, but supporting a linear fallback allows deployment on MVP-only environments (e.g. Cloudflare Workers). Decoupling AST from codegen backend makes dual-target compilation feasible.
* **Pre-emission Validation:** Validating Wasm module structure before binary emission prevents obscure "unknown opcode" or type section errors at runtime/deploy time, accelerating development loop and ensuring spec compliance.

