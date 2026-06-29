# Product Specification: Direct Gleam-to-Wasm GC Compiler

This specification defines the product requirements, design guidelines, type mappings, and testing strategies for a Direct Gleam-to-WebAssembly compiler targeting the **Wasm Garbage Collection (Wasm GC)** proposal.

---

## 1. Specification Index

To maintain modularity and comply with the codebase line limit guidelines (<250 lines per file), the specification is divided into the following dedicated sections:

1. **[Vision & Architecture](file:///Users/moe/Desktop/gleamwasm/docs/spec_vision_architecture.md)**
   * Core design goals (no custom GC runtime, lightweight binaries).
   * High-level compiler flow and Rust-based toolchain integration.
2. **[Type Mapping Specification](file:///Users/moe/Desktop/gleamwasm/docs/spec_type_mapping.md)**
   * Compilation of Gleam primitives (Int, Float, Bool, Nil).
   * Representation of Algebraic Data Types (ADTs) using Wasm GC subtyping.
   * Closure and function compilation patterns.
3. **[Host FFI & String Representation](file:///Users/moe/Desktop/gleamwasm/docs/spec_ffi_interop.md)**
   * JS String Builtins (`wasm:js-string`) vs. WASI Byte Array strings.
   * Foreign Function Interface (FFI) bindings using `externref`.
4. **[Roadmap & Verification Plan](file:///Users/moe/Desktop/gleamwasm/docs/spec_verification_roadmap.md)**
   * Integration testing harness using Red/Green TDD.
   * Binary size optimizations (`wasm-opt`) and performance targets.

---

## 2. Key Design Decisions

> [!NOTE]
> By targeting the native Wasm GC proposal instead of embedding a custom runtime like GC-marked linear memory allocator, the final generated `.wasm` binary size drops by over 90% (from ~100KB+ baseline down to <5KB).

> [!IMPORTANT]
> To support standard functional paradigms, compiling Gleam's recursion-heavy loops requires the native WebAssembly **Tail Call** instruction set (`return_call`).

---

## 3. High-Level Performance Matrix

| Target | Binary Size Baseline | GC Pause Profile | JS Interop Cost | Stack Depth |
| :--- | :--- | :--- | :--- | :--- |
| **JS Target** | Source Size (Code only) | Shared with Browser | Zero (Same VM) | Limited by JS Call Stack |
| **Wasm GC (Web)** | `< 5 KB` | Browser GC Optimized | Minimal (externref) | Unlimited (Tail Calls) |
| **Wasm GC (WASI)** | `< 10 KB` | Host VM Managed | N/A (Standard ABI) | Unlimited (Tail Calls) |
