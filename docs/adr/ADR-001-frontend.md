# ADR-001: Gleam frontend strategy

- **Status:** PROPOSED — awaiting owner decision (plan step 8 gate)
- **Date:** 2026-10-21
- **Deciders:** moe (owner), OpenCrabs (analysis + spike)
- **Evidence:** spike commit `77bdfb9`; numbers table in `docs/gap_analysis_v2.md` §5

## Context

gleam-wasm's `parse.rs` (now `parse/`) is an I64-only subset parser: it reads a
useful Gleam expression grammar but ignores annotations, rejects string
literals, silently skips imports/consts/custom types, and types everything
`ValType::I64`. The dogfood apps hand-build `TypedExpr` in Rust test code.
Whole-language compilation (`pub fn main() { 42 }` → wasm without Rust glue)
requires a typed frontend: parser + type inference + module resolution.

The question: **compose Gleam's own frontend, or grow our own?**

## Spike measurements (Option A costs, 2026-10-21)

| Measurement | Value |
|---|---|
| Flag-off dep tree | 30 crates, unchanged, default build exit 0 |
| Flag-on dep tree | **636 crates** (~21×) |
| Toolchain pincer | gleam ≥ v1.19.0 uses unstable `Option::map_or_default` (rust-lang#138099) → needs stable > 1.97. No rustup on this machine. v1.18.1 is the newest tag that builds on 1.97. |
| Supply-chain babysitting | gleam declares `ecow = "0"` (wildcard). ecow ≥ 0.3.0 adds ambiguous impls → 4× E0283. Build required pinning ecow to 0.2.6 (gleam's own lock value). gleam-core is **git-only, never published to crates.io**. |
| Parser standalone | ✓ cheap — `parse_module(path, src, &WarningEmitter::null())` ran on `pub fn main() { 42 }`, correct AST |
| Typed AST standalone | ✗ **the killer** — `TypedExpr` requires full package-analysis scaffolding: module analyzer, package interfaces, stdlib resolution. Not a function call; a subsystem. |
| Variant mapping | theirs 27 / ours 16; ~12 map directly; their Bool/Nil are constructors (Var/ModuleSelect), Pipeline arrives pre-desugar; Echo/RecordUpdate/BitArray/Todo/Panic unmapped |
| License | Apache-2.0 — compatible |

## Options

### A1 — In-tree gleam-core feature (as spiked)

`gleam-core` git dep behind `gleam-frontend` feature; lowering layer maps
their `TypedExpr` → our IR inside this crate.

- **Pro:** real typechecker, real parser, tracks Gleam evolution; no
  duplicated essence in-tree.
- **Con:** 636 crates in the feature tree; we inherit gleam's toolchain
  treadmill (already one stable behind), wildcard-dep lock babysitting, and a
  git-only crate as a foundation; package-analysis scaffolding must be built
  anyway to get typed AST.

### A2 — Sidecar extractor (recommended)

A **separate tool** (own repo or quarantined workspace member, flag off by
default) depends on gleam-core, runs the full package analysis, and emits the
typed AST as **data** (JSON/bincode schema). `gleam-wasm` consumes that data
file and stays a pure 30-crate backend.

- **Pro:** composition through data — the Hickey shape. Supply chain and
  toolchain pin quarantined in the tool; gleam-wasm keeps its 2-runtime-dep
  identity; the AST schema is an explicit, versioned contract; the tool can
  upgrade toolchains freely without touching this repo's floor.
- **Con:** the package-analysis scaffolding cost is *relocated, not avoided*
  (it exists in A1 too); the serialization schema is a new boundary to
  maintain; two-step build UX (`gleam-frontend-extract` then `gleam-wasm build`).

### B — Extend the hand-rolled parser with real types

Grow `parse/` into a true frontend: honor annotations, add inference for the
supported subset, strings, custom types, modules/imports, case exhaustiveness.

- **Pro:** zero new deps, full control, honest fit if the product ambition
  stays "CF-worker subset compiler" rather than "Gleam compiler".
- **Con:** re-implements a typechecker's essence — the duplicated-essence trap;
  Gleam's semantics are big (generics, exhaustiveness, shadowing, pipes, bit
  arrays) and every upstream Gleam release widens the divergence; unbounded
  maintenance tail.

## Recommendation

**A2**, with B as the fallback *only if* the CF-subset ambition is the product
and full-language support is explicitly off the roadmap. A1 is rejected: it
imports a product, not a library, and welds this repo's toolchain floor to
gleam's treadmill for no gain over the sidecar.

Rich Hickey reading: A2 complects nothing at runtime — the boundary is data,
which is the simplest thing that can possibly work; B re-implements essence;
A1 imports someone else's complected whole.

## Consequences (if A2 is picked)

1. New sidecar crate/repo with gleam-core + package-analysis scaffolding.
2. Versioned typed-AST schema (start JSON, optimize later).
3. `gleam-wasm build -i ast.json` path added alongside the existing subset parser.
4. Spike artifacts (`gleam-frontend` feature, `examples/spike_frontend.rs`)
   either deleted or moved to the sidecar as its seed.
5. Task 9 (plan) re-scopes to: sidecar MVP extracting the counter app + our
   loader consuming it end-to-end.

## Decision

**PENDING — owner gate.** No implementation (plan task 9) starts until moe
picks A1 / A2 / B.
