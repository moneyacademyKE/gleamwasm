# Gap Analysis: gleamwasm vs gl_wasm + gwr

Comparing our approach against the two main Gleam-Wasm ecosystem projects.

## gl_wasm (gertvv, 34 stars, 52 commits)

**What it is:** A Gleam library for creating binary WebAssembly modules. Pure Gleam, runs on BEAM and JS targets. Written in functional style using Result monad chains.

**Key differences from our approach:**

| Aspect | gl_wasm | gleamwasm |
|--------|---------|-----------|
| Language | Gleam (100%) | Rust |
| Target | Wasm 3.0 draft spec | Wasm GC proposal |
| Module builder | `ModuleBuilder` with Result monad | Direct `Module` struct mutation |
| WAT support | ❌ Non-goal | ✅ Full WAT emitter |
| Binary-only | ✅ Primary output | ✅ Round-trip through wast crate |
| Type system | Gleam ADTs for wasm types | Rust enums mirroring Wasm spec |
| GC types | Partial — struct/array/i31/extern | ✅ Full — subtyping, br_on_cast |
| Tail calls | ✅ return_call | ✅ return_call, return_call_ref |
| Function builder | Stack-safe instruction accumulation | FunctionBuilder helper |
| Validation | Type-level validation at build time | Rely on wast validate() |
| Memory | ✅ Linear memory | ✅ Linear + GC |
| Size | ~20KB library (Gleam compiled) | ~500KB binary (Rust compiled) |

**What gleamwasm should steal from gl_wasm:**

1. **OutputStream abstraction** — gl_wasm has `wasm.OutputStream(stream, write_bytes, close)` which abstracts over file/network/memory sinks. Our `emit_wasm() -> Vec<u8>` should support writing to IO streams.
2. **ModuleBuilder with validation at build time** — gl_wasm validates type indices, function signatures, and instruction sequences as you add them, not at emit time. We validate at parse time (wast round-trip) which is later.
3. **Table, Element, Data, Global sections** — gl_wasm supports all of these (checked in feature list). We only support type/import/function/memory/export/code. Missing: table section for `call_indirect`, global section for mutable globals, data section for pre-initialized memory.
4. **Instruction completeness** — gl_wasm supports i32, i64, f32, f64, v128, numeric conversions, truncations. Our linear target only has a subset of i32/f64 ops. Missing: all float truncation/convert ops, v128 SIMD, i32/i64 saturating ops.
5. **`End` instruction** — gl_wasm treats `End` as a first-class instruction that must be explicitly added. Our encoder auto-appends `0x0B` which caused the workerd validation bug (section size mismatch when manual layering is needed).

**What gleamwasm has that gl_wasm doesn't:**

1. **Rust-level integration** — We can integrate directly with the Gleam compiler's Rust codebase. gl_wasm is a separate library that would need a Gleam→Wasm compiler built on top.
2. **GC subtyping** — Our type mapper maps Gleam ADTs directly to Wasm GC struct subtyping. gl_wasm supports struct types but not the subtyping hierarchy.
3. **Linear memory target** — Cloudflare-compatible output that avoids GC instructions entirely. gl_wasm targets Wasm 3.0 which requires GC support in the runtime.
4. **Pattern match lowering** — Our codegen translates Gleam pattern matching to both `ref.test`/`br_on_cast` (GC) and `i32.eq`/`if` (linear). gl_wasm is a lower-level library — the caller builds instructions.
5. **Expression compiler** — We have `TypedExpr` AST and compilers for both GC and linear. gl_wasm has no expression IR — you build instructions manually.

## gwr (BrendoCosta, 1 star, 123 commits)

**What it is:** A WebAssembly *runtime* (virtual machine) written in Gleam. Parses and executes Wasm bytecode. Targets Erlang. Supports integer arithmetic, function calls, and basic control flow.

**Key differences from our approach:**

| Aspect | gwr | gleamwasm |
|--------|-----|-----------|
| Direction | Wasm → Gleam (runtime/VM) | Gleam → Wasm (compiler) |
| Runtime | Interpreted in Gleam/BEAM | Compiled to native via V8/wasmtime |
| Target | Erlang VM | Browsers, wasmtime, CF Workers, Deno |
| GC types | Not supported (MVP only) | GG core feature |
| Linear memory | Parses memory section | Emits memory section |
| Module parsing | Full wasm parse → internal AST | wast crate parse ← our WAT |
| Test suite | Rust/WAT → wasm → gwr.run() | TDD with TypedExpr → WAT assertions |

**What gleamwasm should steal from gwr:**

1. **Test suite approach** — gwr has a `test_suite/` directory with WAT files compiled to `.wasm` via `wat2wasm`, then loaded and executed. We should have a similar wasmtest suite: WAT fixtures that are compiled, loaded, and executed by wasmtime/CF runtime, with assertions on return values.
2. **Module validation** — gwr validates module structure during parsing. Our hand-rolled binary encoder has no validation pass — bugs (like the section ordering or missing `end` opcode) only surface at runtime. We should add `validate_module()` that checks Wasm well-formedness before emission.
3. **Spec version tracking** — gwr explicitly tracks which spec features it implements. We should add a `SUPPORTED_SPEC.md` listing every Wasm instruction we support, organized by proposal.

**What gleamwasm has that gwr doesn't:**

1. **Compilation direction** — We go Gleam→Wasm. gwr goes Wasm→Gleam execution. Complementary.
2. **GC + tail calls** — Our primary differentiator. gwr is MVP Wasm only.
3. **Production deployment** — We output deployable `.wasm` files. gwr is an experimental VM — not for production use.

## What We're Still Missing (Inspired by Both)

### From gl_wasm (high priority)

1. **Table section support** — Needed for `call_indirect` (virtual dispatch). Required for closures and higher-order functions.
2. **Global section** — Mutable globals for module-level state. Our linear heap pointer hack (stored at address 0) should be a proper global.
3. **Data section** — Pre-initialize linear memory. Needed for string literals and static data.
4. **Element section** — Needed for function tables that reference functions.
5. **OutputStream abstraction** — `emit_wasm(output: &mut dyn Write) -> Result<()>` instead of `-> Vec<u8>`.
6. **Build-time validation** — Validate indices, types, and instruction sequences as they're added to the module, not only at emit time.
7. **Instruction completeness** — Missing i32/i64 saturation ops (`i32.trunc_sat_f64_s` etc.), sign-extension ops, and the `select` instruction.

### From gwr (high priority)

1. **Test suite fixtures** — Directory of WAT test programs that compile to wasm and execute with known outputs. We have functional tests but no wasmtest-style fixtures.
2. **Module validation** — Pre-emission validation pass that checks: all type indices exist, all locals are referenced within bounds, control flow is well-structured, memory accesses are aligned.
3. **Spec coverage documentation** — A file listing every Wasm 3.0 instruction and its support status in our codebase.

### From the Cloudflare Workers runtime itself

1. **Memory growth** — Our memory has `max 256` pages fixed. Should support `memory.grow` for dynamic allocation.
2. **Import resolution** — workerd resolves imports from the JS binding layer. Our linear module doesn't use imports (it's self-contained). But real programs need `fetch`, `console.log`, KV bindings — these come through imports.
3. **JS string interop** — Cloudflare supports `externref` but not GC types. We should export helper functions that convert between JS strings and linear memory byte arrays.

## Actionable Next Steps

| # | What | Source | Priority | Status |
|---|------|--------|----------|--------|
| 1 | Add global section (replace heap-pointer-at-0 hack) | gl_wasm | High | ✅ DONE |
| 2 | Add pre-emission module validation | gwr | High | ✅ DONE |
| 3 | Add missing i32/i64/f64 instructions to linear target | gl_wasm | Medium | ✅ DONE |
| 4 | Create wasmtest fixture directory | gwr | Medium | ✅ DONE |
| 5 | Add OutputStream abstraction | gl_wasm | Low | 📋 Pending |
| 6 | Add data/element/table sections | gl_wasm | Low | 📋 Pending |
| 7 | Write SUPPORTED_SPEC.md | gwr | Low | 📋 Pending |
| 8 | Add memory.grow support | CF runtime | Medium | ✅ DONE |

## Implementation Notes

**Item 1 - Global section:** Replaced the heap-pointer-at-linear-memory-address-0 hack with a proper mutable i32 global. The `$alloc` function now uses `global.get 0` / `global.set 0`. WAT and binary emitters both emit the global section correctly.

**Item 2 - Pre-emission validation:** Added `src/validate.rs` with `validate_module()` that checks: type indices exist, local.get/set/tee are in bounds, call/return_call indices exist, branch depths are valid for label nesting, export function indices exist, GC instructions are not present in linear modules. 5 validation tests pass. The CLI and `compile_module` call validation before emission.

**Item 3 - Missing instructions:** Added to IR/Display/binary encoder: `GlobalGet`, `GlobalSet`, `I64Load`, `I64Store`, `I32And`, `I32Or`, `I32Xor`, `I32Shl`, `I32ShrS`, `I32ShrU`, `Select`, `MemoryGrow`, `MemorySize`, `F64ConvertSI32`, `F64Trunc`, `I32TruncSatF64S`.

**Item 4 - Wasmtest fixtures:** Created `wasmtest/` directory with 3 fixtures: `add.wat` (i64 arithmetic), `if_then_else.wat` (control flow), `memory.wat` (linear memory store/load).

**Item 8 - Memory grow:** Added `MemoryGrow` and `MemorySize` instructions with full binary encoder support.

## Current State

- **75 tests across 21 suites, zero warnings, clean clippy**
- Section ordering: 1 (type) → 2 (import) → 3 (function) → 5 (memory) → 6 (global) → 7 (export) → 10 (code)
- Global section emitted correctly in both WAT and binary formats
- Pre-emission validation catches: invalid locals, invalid calls, invalid branches, GC-in-linear, invalid exports
