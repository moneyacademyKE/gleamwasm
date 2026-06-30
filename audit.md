# Rich Hickey Audit: Gleam-to-Wasm GC Compiler

Gap analysis and bug inventory. **All 16 items resolved.** 92 tests, 24 suites, clean clippy.
Reviewed against three real Gleam apps (Lustre Counter, BEAM CMS, GleamUnison v3.4.1).

---

## Critical Bugs (RESOLVED)

### BUG-1 ✅: Match `br 0` creates infinite loop
**File:** `src/codegen/expr.rs:171`  
**Fix:** Changed `br 0` → `br 2`. Skips the `then` block (depth 0), skips the `If` instruction (depth 1), lands at `match_exit` block (depth 2).  
**Impact was:** Every multi-variant match diverged at runtime.

### BUG-2 ✅: Wasmtime tests pass no arguments
**File:** `tests/wasmtime_runtime.rs`  
**Fix:** Rewrote tests with wrapper functions embedding concrete arguments (`add(10, 32)`, `factorial(5)`) directly in WAT.

### BUG-3 ✅: Hardcoded type indices in `build_web_imports()`
**File:** `src/ffi/imports.rs`  
**Fix:** Signature changed to accept indices from `register_web_builtins()`.

### BUG-4 ✅: One-armed `If` with result type violates WASM spec
**File:** `src/emit/wasm.rs`  
**Fix:** Binary encoder emits `0x40` (empty blocktype) for one-armed `If` regardless of `then_branch.result`. Two-armed `If` uses the result type consistently. Workerd validator caught this.

### BUG-5 ✅: Signed LEB128 encoding for negative numbers
**File:** `src/emit/wasm.rs`  
**Fix:** Changed `encode_i32` from unsigned LEB128 to signed LEB128 (`encode_sleb128`). `I32Const(-1)` now emits 1 byte (`0x7F`) instead of 5 bytes.

---

## Design Gaps (RESOLVED)

### GAP-1 ✅: No module-level compilation entry point
**Fix:** `compile_module_with_opt()` + `compile_to_linear()` + `compile_gleamunison()` + `compile_self_contained()`. Multiple compilation targets.

### GAP-2 ✅: Dead `GleamExpr` AST
**Fix:** Removed.

### GAP-3 ✅: Closure codegen stub
**Fix:** Extracts param types from closure definition, registers proper closure struct with function type reference.

### GAP-4 ✅: No Int/Float boxed type registration
**Fix:** `register_boxed_primitives()` registers `$Int` and `$Float`.

### GAP-5 ✅: Match binding locals assume I64
**Fix:** TypeMapper tracks `variant_fields: HashMap<u32, Vec<ValType>>`.

### GAP-6 ✅: Dual local tracking between FunctionBuilder and TypeMapper
**Fix:** `compile_function()` bridge reconciles locals through `compile_module` integration point.

### GAP-7 ✅: Cloudflare Workers linear memory target
**Fix:** Full linear memory codegen in `src/codegen/linear/`. Bump allocator, tagged values, ADT match via `i32.eq`. Verified on wrangler.

### GAP-8 ✅: Self-contained WASM stubs
**Fix:** 17 pure WASM builtins in `src/codegen/gleamunison_sc.rs`. FNV-1a hash, linear KV store, memcpy, hex encode/decode. Zero JS imports.

### GAP-9 ✅: Pre-emission module validation
**Fix:** `src/validate.rs` catches: invalid locals, invalid calls, invalid branches, GC-in-linear, invalid exports, invalid function types.

### GAP-10 ✅: Global section (replaces heap-pointer-at-0)
**Fix:** Mutable i32 global for heap pointer. `$alloc` uses `global.get/set`. Proper section 6 emission in WAT and binary.

---

## Testing Gaps (RESOLVED)

### TEST-1 ✅: No negative/error tests
**Fix:** `tests/negative_tests.rs` with `#[should_panic]` for missing locals/functions.

### TEST-2 ✅: No multi-function integration test
**Fix:** Multi-function module tests in GC and CF targets.

### TEST-3 ✅: Wasmtime tests skip silently
**Fix:** Tests assert correct computed values. Skip only when wasmtime unavailable.

### TEST-4 ✅: No wrangler smoke tests
**Fix:** 4 dogfooded apps deployed + verified via wrangler dev: Lustre counter (186 bytes), Gleam CMS (270 bytes), GleamUnison CF adapter, self-contained GleamUnison.

### TEST-5 ✅: No Cloudflare-specific tests
**Fix:** `tests/cloudflare_target.rs` (4 tests) + `tests/dogfood_cf.rs` (2 tests) + `tests/dogfood_cms.rs` (5 tests) + `tests/dogfood_gleamunison_cf.rs` (3 tests) + `tests/dogfood_gleamunison_sc.rs` (4 tests).

---

## Summary

| Severity | Found | Fixed |
|----------|-------|-------|
| Critical bugs | 5 | 5 |
| Design gaps | 10 | 10 |
| Testing gaps | 5 | 5 |
| **TOTAL** | **20** | **20** |

**92 tests passing across 24 test suites. Zero warnings. Clean clippy. Verified on Cloudflare Workers via wrangler dev.**

## Key Fix Locations

- **BUG-1 (br depth):** `src/codegen/expr.rs` — `br 2` not `br 0`
- **BUG-4 (one-armed if):** `src/emit/wasm.rs` — encode 0x40 for one-armed If
- **BUG-5 (LEB128):** `src/emit/wasm.rs` — `encode_sleb128()` for signed i32
- **GAP-7 (CF target):** `src/codegen/linear/mod.rs` + `expr.rs`
- **GAP-8 (self-contained):** `src/codegen/gleamunison_sc.rs` — 17 pure WASM stubs
- **GAP-9 (validation):** `src/validate.rs` — pre-emission module validation
- **GAP-10 (globals):** `src/codegen/linear/mod.rs` — global section for heap pointer
