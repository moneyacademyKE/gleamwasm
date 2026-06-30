# GleamUnison → Cloudflare WASM Dogfooding

GleamUnison (gleamunison v3.4.1) is a content-addressed language runtime on the
BEAM. It's an ecosystem-scale app with a parser, type checker, compiler backend,
effects runtime, REPL, and HTTP server — all in Gleam.

## App Architecture

| Component | Lines | Description |
|-----------|-------|-------------|
| `parser.gleam` | ~350 | S-expression parser + surface term builder |
| `types.gleam` | ~280 | Type checker cache + handler validation |
| `compile.gleam` | ~180 | Compiler backend: Gleam term → Erlang source |
| `codebase.gleam` | ~250 | Content-addressed code storage + lookup |
| `ast.gleam` | ~120 | Core language AST (Term, Type, Ability) |
| `identity.gleam` | ~80 | Content-hash identity (Ref, Local) |
| `pipeline.gleam` | ~240 | Parse → elaborate → typecheck → compile → run pipeline |
| `effects.gleam` | ~230 | Algebraic effects handler runtime |
| `repl_eval.gleam` | ~200 | REPL evaluation with cache |
| `http.gleam` | ~200 | Cowboy web server wrapper |
| `storage.gleam` | ~320 | JSON file system persistence (21 FFI calls) |
| + 12 more modules | ~1,800 | Level test suites, JSON, crypto, datetime, templates |

**4,271 lines total across 22+ Gleam modules** (not counting the 34 `.erl` files)

## Why It Can't Compile to WASM Today

### BLOCKER: 75 ERTS-dependent FFI calls

Every module depends on `@external(erlang, ...)` calls that are fundamental,
not just IO wrappers:

- **Process dictionary state** (`ffi_state_get`, `ffi_state_set`) — BEAM process dictionary
- **Concurrent evaluation** (`ffi_spawn_concurrent_evals`) — BEAM process spawning
- **Dynamic code loading** (`ensure_loaded`, `compile_source`) — Hot code loading
- **BitArray/JOSE crypto** (`jose_sign`, `jose_verify`) — Erlang crypto NIFs
- **Date/time** (`posix_to_micro`, `utc_timestamp`) — Erlang calendar
- **ETS storage** (`ets_lookup`, `ets_insert`, `ets_delete`) — Erlang term storage
- **Template rendering** — Erlang template engine

### BLOCKER: Runtime model mismatch

GleamUnison is a **self-evaluating language runtime** — it parses, type-checks,
compiles, and loads code at runtime. This requires:

1. Dynamic code generation (→ needs a compiler in the WASM binary)
2. Hot module loading (→ `code:load_binary/3` or equivalent)
3. Mutable process dictionary (→ BEAM process heap)
4. Concurrent processes (→ BEAM scheduler)

None of these exist in the Cloudflare Workers WASM environment (a single-threaded
V8 isolate with MVP Wasm only).

### BLOCKER: External IO conventions

The storage module alone has 21 FFI calls for file system operations (read,
write, list, delete). Cloudflare Workers has no filesystem — data goes through
KV, R2, or D1 bindings (all accessed via JS fetch bridge).

## What WOULD Compile

If we strip out the BEAM-specific layers, these pure functions could compile:

| Function | What it does | WASM-compatible |
|----------|-------------|-----------------|
| `ast.pretty_print()` | Format AST to string | ✅ |
| `ast.hash_of_definition()` | Content hash computation | ✅ (SHA-256) |
| `types.empty_cache()` | Empty Dict | ✅ |
| `types.validate_handler()` | Pure handler validation | ✅ (no dict usage) |
| `lexer.tokenize()` | String → tokens | ✅ |
| `parser.parse_sexpr()` | Tokens → sexprs | ✅ (recursion) |
| `parser.sexpr_to_term()` | Sexpr → surface term | ✅ |
| `elab_types` module | Type elaboration | ✅ |
| Individual math levels | Int/float/list operations | ✅ |

## What Needs Stub Imports

To make the app deployable, every `@external(erlang, ...)` call needs a
Cloudflare-side JS implementation imported via the WASM import section:

| Erlang FFI | Cloudflare Replacement |
|------------|----------------------|
| `ffi_state_get/set` | KV namespace binding |
| `ffi_spawn_concurrent_evals` | `ctx.waitUntil()` or parallel fetches |
| `ensure_loaded` | KV-stored precompiled modules |
| `ffi_crypto_sign/verify` | Web Crypto API via `externref` |
| `ffi_file_read/write` | KV `put()`/`get()` |
| `ffi_timestamp` | `Date.now()` via import |
| `ffi_eval_expression` | The WASM module itself (eval compiled to wasm) |

## The Gleam Source Parser Problem

gleamwasm currently compiles `TypedExpr` (hand-crafted Rust AST), not `.gleam`
source. To compile GleamUnison's 4,271 lines:

1. **Integrate `gleam-core`** as a Rust dependency for `parse_module()` +
   `infer_module()`
2. **Write a Gleam typed AST → TypedExpr lowering pass**

This is non-trivial — Gleam's typed AST includes modules, imports, type
definitions, pattern matching with guards, pipe operator, `use` expressions,
`case` with multiple clauses, record construction/access, and custom types.
Each feature needs a lowering rule.

## Estimated Integration Effort

| Phase | Work | Effort |
|-------|------|--------|
| 1. `gleam-core` dependency | Cargo.toml + parse_module call | Small |
| 2. Type lowering | Gleam types → Wasm GC structs | Medium |
| 3. Expression lowering | Gleam typed AST → TypedExpr | Large |
| 4. Pattern match lowering | Multi-clause + guards | Medium |
| 5. Import stubs | JS bridge for 75 FFI calls | Large |
| 6. Runtime ports | KV, crypto, fetch imports | Medium |

## Bottom Line

**GleamUnison can't compile to WASM today** — not because gleamwasm is broken,
but because the app is fundamentally BEAM-bound (75 FFI calls to OTP runtime
services + hot code loading). 

What we CAN do: extract the pure-functional core (parser, type checker, AST
manipulation) and compile those individual functions. The parser alone (~350
lines of pure Gleam with no FFI) would make an excellent dogfood target.

Want me to extract and compile the parser or a subset of the pure levels?
