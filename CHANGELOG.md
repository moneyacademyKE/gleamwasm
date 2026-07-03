# Changelog

## v0.2.0 - 2026-07-03

### Added

- Source parser support for normal list literals, multiple `let` statements in function bodies, and generic return type annotations such as `-> List(Int)`.
- Regression test suite covering parser fixes, local allocation, inter-function calls, Cloudflare exports, and Wasm encoding edge cases.
- Expanded dogfood coverage with five generated Gleam apps that scale from 8 to 128 stage functions.
- Cloudflare-compatible dogfood builds for the generated stress suite.
- Desktop GleamUnison dogfood artifacts and compatibility harness notes.

### Fixed

- Function index registration in GC codegen so inter-function calls target the intended function.
- `let` local allocation after params and through variable shadowing.
- Closure preprocessing inside match case bodies.
- Cloudflare/linear export indices for user functions.
- Cloudflare/linear normal function calls, which previously lowered to `unreachable`.
- Cloudflare/linear tail-call target resolution for named functions.
- Manual Wasm encoder output for nullable `none` reference types.

### Verified

- `cargo test` passes across the full suite.
- Generated Cloudflare Wasm exports linear memory and avoids GC WAT markers.
- Generated Wasm artifacts have valid `\0asm` headers.

## v0.1.0

- Initial direct Gleam-to-Wasm compiler prototype with GC and Cloudflare-compatible linear-memory targets.
