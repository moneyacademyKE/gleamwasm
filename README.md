# Gleam-to-Wasm GC Compiler (gleamwasm)

Direct Gleam-to-WebAssembly compiler targeting the **Wasm Garbage Collection (Wasm GC)** proposal and **Cloudflare Workers (MVP Wasm)**. Produces lightweight `.wasm` binaries with zero custom GC runtime overhead.

## Status

**Active development** — Core compiler infrastructure complete. All codegen paths implemented: GC target (struct subtyping + `ref.test`/`br_on_cast`), linear memory target (bump allocator + tagged values). 92 TDD tests passing across 24 test suites. Verified on Cloudflare Workers via local wrangler.

## Architecture

```
gleam-wasm/
├── src/
│   ├── cli.rs                    # CLI interface (--target wasm-web|wasm-wasi|wasm-cf)
│   ├── ir/
│   │   ├── types.rs              # Wasm GC type system (Instr, ValType, HeapType, Block, Local)
│   │   ├── module.rs             # Module (types, imports, functions, exports, memory, globals)
│   │   └── function.rs           # WAT Display impl + Local/LocalKind
│   ├── codegen/
│   │   ├── ast.rs                # TypedExpr — Gleam typed AST IR
│   │   ├── types.rs              # TypeMapper — Gleam types → Wasm GC struct types
│   │   ├── compile.rs            # GleamModule → CompileOutput (GC target)
│   │   ├── expr.rs               # Expression compiler (GC target)
│   │   ├── linear/
│   │   │   ├── mod.rs            # compile_to_linear — Cloudflare MVP Wasm codegen
│   │   │   └── expr.rs           # Linear expression compiler + ADT match lowering
│   │   ├── gleamunison_cf.rs     # GleamUnison CF adapter (12 JS import stubs)
│   │   └── gleamunison_sc.rs     # Self-contained GleamUnison WASM (17 pure WASM stubs)
│   ├── emit/
│   │   ├── wasm.rs               # Binary WASM encoder (section-order: type→import→func→mem→global→export→code)
│   │   └── wat.rs                # WAT text emitter
│   ├── ffi/
│   │   └── imports.rs            # JS String Builtins & WASI byte-array string types
│   ├── validate.rs               # Pre-emission module validation (local bounds, call indices, branch depth)
│   └── wasm_opt.rs               # Binaryen/wasm-opt integration
├── wasmtest/                     # Wasmtest fixtures (add, if_then_else, memory)
├── dogfooding/
│   ├── lustre-counter/           # Lustre MVU counter → CF WASM (186 bytes)
│   │   └── cf-deploy/            # Wrangler deployment script
│   ├── gleam_cms/                # BEAM Gleam CMS → CF WASM (270 bytes, 5 endpoints)
│   │   └── cf-deploy/
│   └── gleamunison/              # GleamUnison v3.4.1 → CF WASM analysis + adapter
│       ├── ANALYSIS.md           # Full architecture analysis (4,271 LOC, 75 FFI calls)
│       └── cf-deploy/            # Self-contained adapter (0 JS imports)
└── tests/
    ├── ir_display.rs             # Instruction Display formatting (7 tests)
    ├── type_mapping.rs           # TypeMapper ADT/closure/primitive registration (3 tests)
    ├── wat_emission.rs           # WAT output correctness (2 tests)
    ├── codegen_primitives.rs     # Primitive expression compilation (7 tests)
    ├── codegen_control_flow.rs   # Tail calls, closures, structs (4 tests)
    ├── codegen_match.rs          # ADT pattern matching (3 tests)
    ├── ffi_interop.rs            # Web builtins, WASI strings, externref (4 tests)
    ├── integration_list_ops.rs   # List map/filter/fold + Result/Option (2 tests)
    ├── binary_size.rs            # Size benchmarks for Hello World, prelude (6 tests)
    ├── cloudflare_target.rs      # CF target: memory export, allocator, no GC, size bound (4 tests)
    ├── dogfood_lustre.rs         # Lustre counter: zero-field variants, match, update (4 tests)
    ├── dogfood_cf.rs             # Lustre counter → CF WASM (2 tests)
    ├── dogfood_wasm_output.rs    # Binary WASM output + magic check (1 test)
    ├── dogfood_cms.rs            # CMS: init, add_page, find_page, publish_page, full deploy (5 tests)
    ├── dogfood_gleamunison.rs    # GleamUnison: local_var_index, range, hash, level1, full (5 tests)
    ├── dogfood_gleamunison_cf.rs # GleamUnison CF adapter (3 tests)
    ├── dogfood_gleamunison_sc.rs # Self-contained GleamUnison (4 tests)
    ├── snapshots.rs              # Insta snapshot tests (2 tests)
    ├── fuzz.rs                   # Proptest fuzzing (3 tests)
    ├── negative_tests.rs         # Error/panic tests (3 tests)
    └── wasmtime_runtime.rs       # wasmtime runtime execution (3 tests)
```

## Supported Features

| Feature | GC Target | CF Target |
|---------|:---------:|:---------:|
| Int, Float, Bool, Nil | ✅ | ✅ |
| Binary operators (i32/i64/f64) | ✅ | ✅ |
| Let bindings | ✅ | ✅ |
| If-then-else | ✅ | ✅ |
| Tail calls (`return_call`) | ✅ | ✅ |
| ADT variant construction | ✅ | ✅ (tagged) |
| ADT pattern matching | ✅ (`ref.test`) | ✅ (`i32.eq`) |
| Closures | ✅ | — |
| Tuples | ✅ | — |
| String literals | ✅ | — |
| List (Cons/Nil) | ✅ | — |
| WAT text emission | ✅ | ✅ |
| WAT ↔ binary round-trip | ✅ (wast crate) | ✅ (hand-rolled) |
| JS import stubs | ✅ | ✅ |
| Self-contained WASM (zero JS) | ✅ | ✅ |
| Memory section + export | ✅ | ✅ |
| Global section | ✅ | ✅ |
| FNV-1a hash (pure WASM) | — | ✅ |
| Linear memory KV store | — | ✅ |
| Byte-level memcpy | — | ✅ |
| Cloudflare Workers verified | — | ✅ |
| Wrangler dev deploy | — | ✅ |
| wasmtime execution | ✅ | — |
| Snapshot testing (insta) | ✅ | ✅ |
| Fuzz testing (proptest) | ✅ | ✅ |
| Pre-emission validation | ✅ | ✅ |
| GitHub Actions CI | ✅ | ✅ |

## Quick Start

```bash
cargo build
cargo test                 # 92 tests across 24 suites
cargo fmt --check
cargo clippy -- -D warnings
```

## Dogfooding Results

| App | Description | Size | Imports | Status |
|-----|-------------|------|---------|--------|
| Lustre Counter | MVU counter app | 186 bytes | 0 JS | ✅ Verified on wrangler |
| Gleam CMS | 4 CRUD endpoints | 270 bytes | 0 JS | ✅ Verified on wrangler |
| GleamUnison CF | 12 FFI stubs (JS) | ~3KB | 12 JS | ✅ Verified on wrangler |
| GleamUnison SC | 17 pure WASM stubs | ~3KB | 0 JS | ✅ Verified on wrangler |

## Key Design Decisions

- **ADR-001**: Direct Wasm GC (no custom GC runtime — sub-5KB binaries)
- **ADR-002**: Dual-mode strings — `externref` for web, `(array i8)` for WASI
- **ADR-003**: ADTs via struct subtyping + `br_on_cast` pattern matching
- **CF Target**: Linear memory + bump allocator + tagged values (zero GC instructions)
- **Self-contained**: All stubs in pure WASM — zero JS imports needed

## References

- [Production Readiness Criteria](production_readiness.md)
- [Rich Hickey Audit](audit.md)
- [Cloudflare Roadmap](CLOUDFLARE_ROADMAP.md)
- [Gap Analysis vs gl_wasm + gwr](GAP_ANALYSIS.md)
- [Remaining Gaps Analysis (v2)](file:///Users/moe/Desktop/gleamwasm/docs/gap_analysis_v2.md)
- [Dogfooding Report](DOGFOOD.md)
- [GleamUnison Analysis](dogfooding/gleamunison/ANALYSIS.md)
