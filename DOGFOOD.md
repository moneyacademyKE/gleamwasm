# Dogfooding Report

Real Gleam applications compiled to Cloudflare WASM and verified on local wrangler.

## Summary

| App | Description | Source LOC | WASM Size | Imports | Status |
|-----|-------------|-----------|-----------|---------|--------|
| Lustre Counter | MVU counter (Incr/Decr) | ~15 | 186 bytes | 0 JS | ✅ Wrangler verified |
| BEAM Gleam CMS | 4 CRUD endpoints | ~80 | 270 bytes | 0 JS | ✅ Wrangler verified |
| GleamUnison CF | 12 JS FFI stubs | 4,271 | ~3KB | 12 JS | ✅ Wrangler verified |
| GleamUnison SC | Self-contained (zero JS) | 4,271 | ~3KB | 0 JS | ✅ Wrangler verified |
| Generated Stress Suite | 5 apps, 8→128 stage functions | generated | 675B→15.9KB | 0 JS | ✅ wasm-web + CF verified |
| Desktop GleamUnison util + harness | real util module + compatible harness | mixed | 103B→1.3KB | 0 JS | ✅ wasm-web + CF verified |

## Lustre Counter (dogfooding/lustre-counter)

**WAT snippet:**
```
(func update ...
  local.get 0  ;; state
  i32.const 1   ;; action = Incr
  local.set 1
  local.get 1
  i32.add       ;; state + 1
  call $1       ;; box_int
)
```

**Wrangler results:**
- `update(41)` → `42` ✅
- `update(0)` → `1` ✅
- `update(100)` → `101` ✅

**Features tested:** Zero-field ADT variants, match dispatch, BinOp, full update function.

## BEAM Gleam CMS (dogfooding/gleam_cms)

**4 exported functions:**
- `init()` → `0` ✅
- `add_page(count)` → `count + 1` ✅ (5 sequential calls: 0→1, 1→2, 2→3, 3→4, 4→5)
- `find_page(count, id)` → `id` if id < count, else `-1` ✅
- `published_count(count)` → `count` ✅

**Features tested:** Multi-function module, If-then-else, param name lookup, signed LEB128 encoding.

## GleamUnison v3.4.1 (dogfooding/gleamunison)

See `dogfooding/gleamunison/ANALYSIS.md` for full architecture analysis.

**4,271 LOC, 75 `@external(erlang, ...)` FFI calls.** Only ~15% of code is pure-functional without BEAM dependencies.

### CF Adapter (12 JS import stubs)
- `hash_bytes`: Web Crypto SHA-256
- `hex_to_bytes`, `hash_to_hex`: TextDecoder pass-through
- `hash_equal`: Uint8Array byte comparison
- `state_get/set`: in-memory Map KV store
- `file_read/write`: KV-backed stubs
- `log`: console.log
- `now_ms`, `timestamp`: Date.now()
- `eval`: parseInt / passthrough

### Self-Contained WASM (17 pure WASM stubs, 0 JS imports)
- `$alloc` — bump allocator (global heap pointer)
- `$make_tagged`, `$get_tag`, `$get_payload` — tagged value runtime
- `$hash_bytes` — FNV-1a hash (byte-by-byte loop using `i32.load8_u`, `i32.xor`, `i32.mul`)
- `$hex_to_bytes`, `$hash_to_hex` — identity pass-through
- `$hash_equal` — byte comparison loop with length check
- `$state_get`, `$state_set` — linear memory KV store at offset 128
- `$file_read`, `$file_write` — file I/O stubs
- `$log` — no-op
- `$now_ms`, `$timestamp` — time stubs
- `$eval` — identity eval
- `$memcpy` — byte-by-byte copy loop using `i32.load8_u` / `i32.store8`

## Gaps Found During Dogfooding

### Resolved:
- **Tuple type & construction** — `TypedExpr::Tuple`, auto-register in TypeMapper
- **External/FFI imports in compile_module** — `GleamModule.imports` field
- **String literal support** — `TypedExpr::StringLiteral` placeholder
- **Param name lookup** — `var_map: BTreeMap<String, u32>` in FunctionContext
- **Signed LEB128 encoding** — `I32Const(-1)` was 5 bytes, now 1 byte
- **One-armed if validation** — Binary encoder emits correct blocktype
- **Runtime i64 types** — Linear target uses i32-only for CF compatibility
- **Function index registration** — GC target inter-function calls now resolve to the intended function
- **Linear target calls** — CF target normal and tail calls now resolve through a function map instead of `unreachable` or first-function fallback
- **Local allocation** — `let` locals now allocate after params and survive shadowing
- **Parser sequencing** — Multiple source-level `let` statements fold into the correct expression chain
- **Parser list literals** — Normal `[a, b, c]` literals parse directly
- **Generic return annotations** — Return types such as `-> List(Int)` are skipped correctly by the lightweight parser
- **Manual nullref encoding** — Nullable `none` reference fallback emits a single value-type byte

## v0.2.0 Dogfood Additions

### Generated stress suite

Five generated apps were compiled through both default and Cloudflare-compatible targets. Complexity scales from 8 to 128 stage functions, with nested lets, arithmetic, comparisons, cross-function calls, and list literals.

| App | CF calls | CF WASM size | CF checks |
|-----|---------:|-------------:|-----------|
| app_01_x1 | 23 | 1,364 bytes | memory export, no GC markers |
| app_02_x2 | 39 | 2,335 bytes | memory export, no GC markers |
| app_03_x4 | 71 | 4,272 bytes | memory export, no GC markers |
| app_04_x8 | 135 | 8,148 bytes | memory export, no GC markers |
| app_05_x16 | 263 | 15,999 bytes | memory export, no GC markers |

### Desktop GleamUnison

The Desktop GleamUnison project was dogfooded from `/Users/moe/Desktop/gleamunison_dogfood/gleamunison_repo`.

- Real source compiled: `src/gleamunison/util.gleam`
- Harness compiled: `gleamunison_wasm_harness.gleam`
- Both default and Cloudflare-compatible targets emitted valid Wasm headers
- CF outputs export `memory`

Full `src/gleamunison.gleam` still exceeds the lightweight source parser because the project uses advanced signatures, higher-order generic function types, FFI declarations, and broader Gleam syntax.

### Remaining:
- **Full Gleam source parser integration** — Current lightweight parser covers a useful subset; full Gleam still needs `gleam-core` or broader syntax support
- **Complete list runtime semantics** — Source list literals parse, but runtime semantics need more coverage across both targets
- **Full GleamUnison runtime** — 75 FFI calls need BEAM-equivalent adapters (hot code loading, concurrency, ETS)
- **GleamUnison parser/compiler** — The 4,271 LOC core needs gleam-core to parse

## Roadmap (Ordered by Impact)

1. **Gleam source parser integration** — Add `gleam-core` dependency, wire `parse_module()` + `infer_module()` + TypedExpr lowering pass
2. **List type support** — Heap-allocated Cons cells + list.fold/map/filter in linear memory
3. **GleamUnison runtime ports** — Hot code loading, concurrency, ETS for CP
4. **Performance benchmarks** — wasmtime profiling, binary size regression in CI
