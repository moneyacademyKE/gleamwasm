# Rich Hickey Gap Analysis (v2): Remaining Compiler Gaps

This gap analysis systematically identifies the remaining gaps between the current `gleamwasm` compiler prototype and a production-ready, feature-complete Gleam-to-Wasm compiler.

---

## 1. Feature Gap Matrix

| Feature Area | Current Implementation / Spec | Production Requirement | Gap Severity |
| :--- | :--- | :--- | :--- |
| **Closure Dispatch** | `call_indirect` via global `funcref` table (ADR-004) | Typed function references (`call_ref`) | **Medium** (Performance bottleneck) |
| **Linear Memory GC** | Basic bump allocator (`$alloc`), no deallocation | Automatic memory management (RC or copying GC) | **Critical** (Long-running leaks) |
| **Source Integration**| Hand-rolled `GleamModule` AST structure | Direct integration with `gleam-core` parser | **Critical** (Not a usable compiler yet) |
| **List Representation**| Emulated arrays (partial) | Recursive GC structs (Wasm GC) / Cons cells (Linear) | **High** (Lists are core to Gleam) |
| **BEAM Concurrency**  | Single-threaded execution only | Lightweight scheduler or Worker pool emulation | **High** (Erlang model parity) |
| **Exception Handling**| VM trap (`unreachable`) | Wasm Exception Handling (`throw`/`catch` blocks) | **Medium** (Graceful error handling) |

---

## 2. Detailed Gap Explanations & Trade-offs

### A. Closure Dispatch: `call_indirect` vs. `call_ref`
* **Current:** ADR-004 mandates indices in a global table called via `call_indirect`.
* **Requirement:** In Wasm GC, closures should store a direct function reference `(ref $FuncSig)` and use `call_ref`.
* **Trade-off:** `call_ref` avoids runtime signature checks and table overhead but cannot run on MVP-only runtimes (which require `call_indirect`).

### B. Linear Memory Garbage Collection
* **Current:** The `--target wasm-cf` backend allocates memory via a bump pointer but never collects.
* **Requirement:** To run in production on Cloudflare Workers, we need a garbage collector embedded in the linear memory runtime.
* **Trade-off:** Implementing a basic copying or reference-counting allocator in Rust/Wasm increases binary size by 10-20KB but prevents memory leaks.

### C. Parser and Compiler Front-End Integration
* **Current:** The CLI relies on manually constructing a `GleamModule` structure in Rust tests.
* **Requirement:** Import `gleam-core`, parse `.gleam` files, run type-checking, and compile the typed AST.
* **Trade-off:** Drastically increases compiler complexity and compilation binary size but is required for user-facing utility.

---

## 3. Complexity vs. Utility

| Proposed Action | Implementation Complexity | Utility / Value | Performance Density | Priority |
| :--- | :--- | :--- | :--- | :--- |
| **Upgrade to `call_ref`** | Medium | High (Up to 3x dispatch speed) | High | High |
| **Add Linear Memory GC** | High | Critical (Production stability) | Medium | High |
| **Integrate `gleam-core`**| Very High | Critical (Usability) | High | Critical |
| **Compile Lists natively**| Medium | High (Idiomatic data structures) | High | High |
| **Wasm Exceptions** | Low-Medium | Medium (Panic recovery) | High | Medium |

---

## 4. Actionable Recommendations

1. **Refine ADR-004:** Update the closure specification to use typed function references and `call_ref` for the Wasm GC target, leaving `call_indirect` only for the linear memory fallback.
2. **Implement Reference Counting for Wasm-CF:** Embed a minimal, static reference-counting runtime inside the linear memory output to automate object lifetime tracking.
3. **Pull `gleam-core` Front-End:** Add `gleam-core` dependency to `Cargo.toml`, mapping its `TypedExpr` AST directly to our compiler's codegen input.
4. **Draft Native List Codegen:** Declare `$List` GC structs in `types.rs` and emit native Cons cell allocation logic.
