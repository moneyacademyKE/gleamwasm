# Rich Hickey Gap Analysis (v2): Remaining Compiler Gaps

This gap analysis systematically identifies the remaining gaps between the current `gleamwasm` compiler prototype and a production-ready, feature-complete Gleam-to-Wasm compiler.

---

> Parser/source-integration status is kept in sync with [`production_readiness.md`](../production_readiness.md) §5 — if you edit one, edit the other.

## 1. Feature Gap Matrix

| Feature Area | Current Implementation / Spec | Production Requirement | Gap Severity |
| :--- | :--- | :--- | :--- |
| **Closure Dispatch** | ✅ `call_ref` via `ref.func` + `$Closure` struct (`$code`/`$env` fields), pre-pass collector for inner function compilation | — | **Resolved** |
| **Linear Memory GC** | ✅ Free-list allocator with `$free` for 8B/12B block recycling; match scrutinees preserved for deallocation | — | **Resolved** |
| **Source Integration**| ⚠️ `.gleam` compiles by default via the **A2 sidecar** (`crates/gleamwasm-frontend`, ADR-001): real gleam-core inference → `gleamwasm-typed-ast/v1` JSON. MVP scope: standalone modules, scalars, let/binops/calls — no imports, custom types, `case`, lists, closures. `--frontend builtin` = zero-dep I64 subset parser (§C, production_readiness.md §5) | Module graph, imports, richer expressions | **Partial** |
| **List Representation**| ✅ `TypedExpr::ListCons`/`ListNil` — GC struct via `$Cons`/`$Nil` subtyping, linear via `$cons`/`$nil` builtins with 12-byte tagged cells | — | **Resolved** |
| **BEAM Concurrency**  | Single-threaded execution only | Lightweight scheduler or Worker pool emulation | **High** (Erlang model parity) |
| **Exception Handling**| ✅ `throw` via `TypedExpr::Panic` + `$ExnTag` struct; WAT/binary encoder support | — | **Resolved** |

---

## 2. Detailed Gap Explanations & Trade-offs

### A. Closure Dispatch: `ref.func` + `call_ref` (Implemented)
* **Implemented:** Closures compile to `$Closure` GC structs with two fields: `$code: (ref $FuncSig)` and `$env: (ref null any)`. A pre-processing pass walks all function bodies to collect closure definitions, register their types, and assign inner function indices. Inner closure bodies are compiled as standalone Wasm functions, allowing `ref.func $inner` to reference them. The closure expression emits `struct.new $Closure` populated via `struct.set` with `ref.func` for the code field and `ref.null any` for the environment.
* **Dispatch:** `TypedExpr::CallClosure` evaluates arguments, evaluates the closure struct, extracts `$code` via `struct.get`, then calls via `call_ref $FuncSig`.
* **Choice:** `call_ref` was chosen over `call_indirect` + table sections for performance (avoids runtime signature checks) and simplicity (no table section or element section needed). The `$Closure` struct directly holds a typed function reference.

### B. Linear Memory Garbage Collection (Implemented)
* **Implemented:** The `$alloc` (index 0) function now checks per-size free lists before bumping the heap pointer. The `$free` (index 1) function pushes blocks onto exact-size free lists — globals 1 (8-byte tagged values) and 2 (12-byte Cons cells). Match scrutinee pointers are preserved via `LocalTee` for future deallocation. This prevents leaks in long-running Cloudflare Worker request chains.

### C. Gleam Source Parser (built-in fallback — subset, syntax only, everything typed I64)

> **Update (ADR-001, implemented):** the DEFAULT `.gleam` path is now the A2 sidecar (`crates/gleamwasm-frontend`): real gleam-core parse + inference behind the `gleamwasm-typed-ast/v1` JSON boundary. The parser described below remains as the zero-dep `--frontend builtin` fallback.
* **Implemented:** A lightweight hand-written recursive-descent parser (`src/parse.rs`) tokenizes Gleam source and produces `GleamModule` structs. Parses: function definitions with `pub fn`, parameter lists, blocks, `let` bindings, binary/arithmetic expressions, function calls, `case`/pattern matching, closures (`fn`), list literals (`[1, 2, 3]`), spread patterns (`..rest`), booleans, integers, floats.
* **Not parsed:** string literals (tokenized, but rejected by the expression parser), imports and `const`/`type` declarations (silently skipped — `GleamModule.imports`/`adt_types` are always empty), custom types, and other modules. Type annotations are tokenized then skipped: every parameter, variable, and expression is hardcoded to `ValType::I64` — no type inference.
* **Design choice:** A custom parser was chosen over importing `gleam-core` (150+ dependencies, 100k+ LOC) to keep binary size minimal and avoid dependency bloat. The parser intentionally omits type-checking — `ValType::I64` is used as a placeholder for all types.
* **CLI integration:** `build_module()` in `lib.rs` detects `.gleam` extension and routes through the parser automatically.

### D. Exception Handling (Implemented)
* **Implemented:** `TypedExpr::Panic` compiles to `throw` with a registered `$ExnTag` struct type (tag index for exception handling). The `Instr::Throw(u32)` IR instruction is emitted in WAT (`throw $tag`) and binary (`0xFB 0x08` with tag type index). Validator checks throw against type index bounds and rejects throw in linear modules.
* **Trade-off:** `catch` blocks (`try_table`) are not yet implemented — panics propagate to the Wasm host runtime. This is sufficient for current use cases (Cloudflare Workers trap on exception).

---

## 3. Complexity vs. Utility

| Proposed Action | Implementation Complexity | Utility / Value | Performance Density | Priority |
| :--- | :--- | :--- | :--- | :--- |
| ~~**Upgrade to `call_ref`**~~ | ✅ Completed | ✅ Implemented | ✅ | Done |
| ~~**Add Linear Memory GC**~~ | ✅ Completed | ✅ Implemented | ✅ | Done |
| **Integrate Source Parser** | ⚠️ A2 sidecar wired (real gleam-core inference, MVP subset); built-in parser = opt-in fallback (§C) | ⚠️ Partial | ⚠️ | ADR-001 decided + implemented |
| ~~**Compile Lists natively**~~ | ✅ Completed | ✅ Implemented | ✅ | Done |
| ~~**Wasm Exceptions**~~ | ✅ Completed | ✅ Implemented | ✅ | Done |
| **BEAM Concurrency** | High | High | Low | Future |

---

## 4. Actionable Recommendations

1. ~~**Refine ADR-004:**~~ ✅ — closures use `ref.func` + `call_ref` with `$Closure` structs. No table section needed.
2. ~~**Implement Reference Counting for Wasm-CF:**~~ ✅ — free-list allocator recycles 8B tagged values and 12B Cons cells via `$free` builtin.
3. **Pull Gleam Source Front-End:** ⚠️ Delivered as A2 sidecar — ADR-001 (2026-10-21) chose composing gleam-core via `crates/gleamwasm-frontend` behind a JSON data boundary; real type inference for standalone modules, `pub fn main() { 42 }` runs end-to-end under wasmtime. Remaining: imports/module graph, custom types, `case`, lists, closures.
4. ~~**Draft Native List Codegen:**~~ ✅ — `TypedExpr::ListCons`/`ListNil` with GC struct subtyping and linear Cons cell builtins.
5. ~~**Wasm Exception Handling:**~~ ✅ — `TypedExpr::Panic` compiles to `throw` with `$ExnTag` struct type.
6. **BEAM Concurrency Model:** Remain single-threaded. Worker pool emulation is the most viable path for future BEAM parity.

---

## 5. Spike: gleam-core frontend cost measurement (ADR-001 input, 2026-10-21)

Measured behind the `gleam-frontend` feature flag (`gleam-core` git dep, tag `v1.18.1`,
rev `4a83802`; throwaway example: `examples/spike_frontend.rs`).

| Measurement | Value |
|---|---|
| Default build (flag off) | exit 0 |
| Normal dep tree (flag off) | **30 crates** — unchanged ✓ |
| Normal dep tree (flag on) | **636 crates** (~21×) |
| Toolchain floor | stable 1.97.0 works for v1.18.1 only; gleam ≥ v1.19.0 uses unstable `Option::map_or_default` (rust-lang#138099) → needs a newer stable |
| Supply-chain pin required | `cargo update -p ecow --precise 0.2.6` (gleam workspace declares `ecow = "0"` wildcard; ecow ≥ 0.3.0 adds ambiguous `AsRef`/`From` impls → 4× E0283 in gleam-core) |
| License | Apache-2.0 — compatible ✓ |
| Parser standalone | ✓ `parse_module(path, src, &WarningEmitter::null()) -> Parsed { module: UntypedModule, extra }` — ran on `pub fn main() { 42 }`, correct AST |
| Typed AST standalone | ✗ `TypedExpr` (ast/typed.rs) requires full package-analysis scaffolding: module analyzer, package interfaces, stdlib resolution. Parse layer yields untyped AST only (`return_type: ()` placeholder pre-inference). |
| Cold flag-on build time | not cleanly measured (session hit run caps); gleam-core alone compiles in minutes, full 636-crate cold build longer |

### TypedExpr variant survey

| gleam-core (27) | gleam-wasm (16) |
|---|---|
| Int Float String Block Pipeline Var Fn List Call BinOp Case RecordAccess PositionalAccess ModuleSelect Tuple TupleIndex Todo Panic Echo BitArray RecordUpdate NegateBool NegateInt Invalid | Int Float Bool Nil Var BinOp Call CallClosure TailCall If Let Closure StructNew StructGet Match Tuple TupleGet |

Mapping notes:
- ~12 variants map directly (Int/Float/String/Var/Call/BinOp/Fn↔Closure/Case↔Match/Tuple/TupleIndex↔TupleGet/List/Block).
- Gleam `True`/`False`/`Nil` are **constructors** (Var/ModuleSelect), not literals — our Bool/Nil are a lowering target, not a source shape.
- `Pipeline` (|>) arrives pre-desugar in typed AST; `Echo`, `RecordUpdate`, `BitArray`, `Todo`, `Panic` have no counterpart in our emitter yet.
- Their nodes carry spans, EcoString, purity, and per-target implementation flags — all surplus to our codegen but free to ignore.

Verdict input for ADR-001: Option A imports a **product**, not a library — wildcard deps,
toolchain treadmill, and the typed pipeline assumes you are building a whole Gleam package.
