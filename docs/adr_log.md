# Architectural Decision Record (ADR) Log

This document records the design and architectural decisions made for the Direct Gleam-to-Wasm GC Compiler.

---

## [ADR-001] Target Wasm GC Proposal Directly
* **Status:** Approved
* **Context:** Gleam requires garbage collection for its data structures. Compiling a custom allocator and GC runtime (e.g. mark-and-sweep) increases binary size by 100KB+ and degrades JS interop.
* **Decision:** Target the WebAssembly Garbage Collection (Wasm GC) proposal directly. The compiler will emit `(struct)` and `(array)` allocations managed by the host VM's native garbage collector.
* **Consequences:** Binaries are extremely lightweight (<5KB baseline). Compilation requires modern Wasm GC engines (Chrome 130+, Firefox 120+, Safari 17.4+).

---

## [ADR-002] Dual-Mode String Compilation Strategy
* **Status:** Approved
* **Context:** String operations on the web are highly optimized when using native JS strings. In standalone WASI runtimes, JS strings do not exist.
* **Decision:** Implement a dual-mode compilation flag. `--target wasm-web` will use `externref` and the JS String Builtins proposal (`wasm:js-string`). `--target wasm-wasi` will compile strings to Wasm GC byte arrays `(array i8)` in UTF-8 format.
* **Consequences:** Maximum performance on the web with zero string copy overhead, while retaining backend portability for standalone edge runtimes.

---

## [ADR-003] ADT Representation via Struct Subtyping
* **Status:** Approved
* **Context:** Gleam custom types (ADTs) have multiple variants that must be instantiated and pattern-matched at runtime.
* **Decision:** Map Gleam custom types to Wasm GC structs using inheritance/subtyping: a base struct type represents the ADT, and variant struct types declare the base type as their supertype. Perform pattern matching using native instructions (`ref.test` and `br_on_cast`).
* **Consequences:** Leverages host VM type information and avoids dynamic tag comparisons.
