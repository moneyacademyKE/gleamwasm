# Rich Hickey Gap Analysis: Direct Gleam-to-Wasm GC Compiler

This gap analysis evaluates the compilation target options for Gleam, comparing the existing target strategies (JavaScript, Erlang/BEAM) against hypothetical WebAssembly compilation strategies (Linear Memory Wasm with a custom GC vs. Wasm GC).

---

## 1. Feature Set Comparison

| Feature / Dimension | JavaScript (Current Web Target) | Erlang/BEAM (Official Target) | Wasm (Linear Memory + Custom GC) | Wasm GC (Proposed Target) |
| :--- | :--- | :--- | :--- | :--- |
| **Garbage Collection** | Host JS GC (V8/JSC/SpiderMonkey) | Process-isolated Heap GC | Custom runtime GC shipped in binary (e.g., Boehm or custom) | Native Host VM GC (Wasm GC) |
| **Compilation Artifact** | JS Source (`.mjs` / `.js`) | BEAM Bytecode (`.beam`) | Wasm Binary (`.wasm` + linear memory allocator) | Wasm Binary (`.wasm` with GC types) |
| **Runtime Overhead** | Minimal (relies on browser runtime) | Large (requires Erlang VM / BEAM) | Medium-High (GC logic + memory allocator included) | Extremely Minimal (host engine provides GC & struct layouts) |
| **Tail Call Optimization** | Engine dependent (limited ES6 support) | First-class tail calls (native BEAM) | Hard/Requires shadow stack or trampolines | First-class tail calls (via Wasm Tail Call extension) |
| **Type Safety at Runtime** | Dynamic (JS is dynamic; types erased) | Dynamic (BEAM term types) | Unsafe/Flat (types flattened to linear addresses) | Statically Typed (checked by Wasm VM verification) |
| **JS/Web Interoperability** | Native / Direct | via Ports/Sockets (Erlang) | Complex (requires memory copying, `textdecoder`, wrappers) | Native (via `externref`, `anyref`, and JS String Builtins) |
| **Binary Size** | Depends on source size | N/A (requires BEAM runtime installation) | Medium to Large (due to embedded GC and stdlib, >100KB) | Very Small (often <10KB for simple modules) |
| **Concurrency Model** | Single-threaded Event Loop (Promises) | Actor Model (preemptive processes) | Threading / Web Workers (manual memory synchronization) | Threading / Web Workers (shared memory or structured message passing) |

---

## 2. Feature Difference Explanations

### A. Garbage Collection (GC)
* **JavaScript:** Leverages the host VM's highly optimized, generational GC. Zero extra payload size, but execution profile is shared with browser layout/main thread and has GC pauses.
* **BEAM:** Uses a per-process heap GC. Highly concurrent and avoids global GC pauses, but is tied to the Erlang VM.
* **Linear Memory Wasm:** Requires compiling a GC engine (like Mark-Sweep or Ref-Counting) into the Wasm binary. This bloats binary sizes and requires the compiler to emit manual allocation/deallocation calls.
* **Wasm GC:** Leverages the host browser's existing garbage collector directly. Custom structures are defined via `(struct)` and `(array)` and tracked natively, avoiding runtime GC bloat and achieving cross-heap reference tracing.

### B. Tail Call Optimization (TCO)
* **JavaScript:** Most engines (except JavaScriptCore/Safari) do not implement ES6 Tail Call Optimization. Compiling recursive functions to JS requires either recursion limits or complex compiler transpilation (like trampolining).
* **Wasm GC + Tail Call Extension:** Leverages native `return_call` and `return_call_indirect` instructions. This guarantees tail-recursive functions (a core primitive in Gleam) execute in $O(1)$ stack space with zero runtime translation overhead.

### C. JS/Web Interoperability & String Representation
* **Linear Memory Wasm:** Passing a string to JS requires copying it out of linear memory into a JS typed array and decoding it.
* **Wasm GC:** With Phase 4 **JS String Builtins** (`wasm:js-string`), Wasm can import native JS string operations. By mapping Gleam `String` to `externref`, strings can be passed between JS and Wasm with zero copies and manipulated using V8-internal string operations.

---

## 3. Benefits and Trade-offs

### Option A: Compile to JavaScript (Status Quo)
* **Benefits:** 100% compatible with existing web ecosystems, zero-copy interop with JS libraries, browser devtools support.
* **Trade-offs:** Types are erased at runtime; TCO is missing or slow; JS execution speed can vary; bundle size grows with large dependency graphs.

### Option B: Wasm via Linear Memory + Custom GC
* **Benefits:** Run-anywhere capability on any basic Wasm interpreter (does not require modern Wasm GC or Tail Call support).
* **Trade-offs:** Heavy binary footprint (GC runtime + allocator); slow and complex interop with JS (constant copying); memory fragmentation risks in linear memory.

### Option C: Direct Compile to Wasm GC (Proposed)
* **Benefits:**
  * **Ultra-Lightweight Binaries:** No custom GC runtime shipped; only application logic.
  * **High Performance:** Native struct allocation and field access; engine-optimized garbage collection.
  * **Type Verification:** Wasm engine verifies the structural types at compile/load time, preventing memory safety bugs.
  * **Native TCO:** Fully matches Gleam's recursion-heavy idiomatic patterns.
  * **Direct Interop:** Zero-copy string/object manipulation using `externref` and JS String Builtins.
* **Trade-offs:** Requires modern Wasm runtimes supporting Wasm GC and Tail Calls (Chrome 130+, Firefox 120+, Safari 17.4+); limited toolchain ecosystem (compilers must generate WAT/Wasm binary directly).

---

## 4. Complexity vs. Utility

| Compile Strategy | Implementation Complexity | Utility / Value | Performance Density | Maintenance Overhead |
| :--- | :--- | :--- | :--- | :--- |
| **JS Transpilation** | Low | High (Web ecosystem) | Medium | Low |
| **Wasm Linear Memory** | Very High (requires writing GC/allocator) | Low (heavyweight binaries) | Low-Medium | High |
| **Wasm GC Direct Compiler**| Medium-High (custom binary output) | Very High (lightweight, fast) | High | Medium |

---

## 5. Actionable Recommendation

**We recommend Option C: Direct Gleam-to-Wasm GC Compiler.**
* **Power & Capabilities:** Native Wasm GC, TCO, and `externref` string builtins provide a modern compilation path that preserves functional programming paradigms (immutability, recursion) without runtime overhead.
* **Speed:** Compiles directly to WAT/Wasm bytecode, skipping JS parser/compiler overhead.
* **Complexity:** Eliminates the massive complexity of implementing a custom runtime allocator or GC, focusing compilation purely on type-to-struct mapping.

### Recommended Implementation Roadmap:
1. **Define Core Type Mappings:** Map Gleam primitives (`Int`, `Float`, `String`, `Bool`) and ADTs (variants/structs) directly to Wasm GC types (`i31ref`, `f64`, `externref`, subtyped `struct`).
2. **Implement Tail Call CodeGen:** Use the WebAssembly Tail Call proposal (`return_call`).
3. **Establish Web Bindings:** Utilize `wasm:js-string` for strings and `externref` for generic JS objects.
4. **Develop a Lightweight Rust-based Code Generator:** Integrate directly into the existing Rust-based Gleam compiler codebase as a new target backend (`src/wasm.rs`).
