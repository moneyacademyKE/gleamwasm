# Gleam-to-Wasm GC Compiler Spec: Host FFI & Strings

This document specifies the interoperability (FFI) layer between the Wasm GC module and JavaScript/Host environments, focusing on string representation and function calls.

---

## 1. Dual-Mode String Representation

To balance performance on the web with portability in standalone WASI runtimes, the compiler supports two compilation modes for `String`:

### Mode A: Web Target (`--target wasm-web`)
* **Underlying Type:** `externref` (referencing a native JavaScript string).
* **Mechanism:** Imports optimized string functions directly from the browser host using the standardized **JS String Builtins** proposal (the `wasm:js-string` builtin module).
* **Benefits:** Zero-copy passing of strings between JS and Wasm; direct usage of JavaScript's highly optimized engine-level string manipulation.

```wat
;; Import JS string concatenation builtin
(import "wasm:js-string" "concat" (func $js_str_concat (param externref externref) (result externref)))
```

### Mode B: WASI Target (`--target wasm-wasi`)
* **Underlying Type:** A Wasm GC-managed array of UTF-8 bytes.
* **Definition:**
  ```wat
  (type $StringArray (array i8))
  (type $GleamString (struct 
    (field $length i32)
    (field $bytes (ref $StringArray))
  ))
  ```
* **Benefits:** Complete platform independence. Runs on any WASI-compliant Wasm GC engine (e.g. Wasmtime) without JS host assumptions.

---

## 2. External Functions (FFI)

Gleam's FFI is declared using `external` blocks.

### Example Gleam Code:
```gleam
@external(javascript, "./dom.js", "write_text")
pub fn write_text(element_id: String, text: String) -> Nil
```

### Compilation Strategy:
* **Imports:** The compiler translates this to a Wasm import.
* **Parameter Conversion:**
  * For `--target wasm-web`, the imported function accepts two `externref` strings.
  * For `--target wasm-wasi`, the import takes array references or pointers, requiring a JS wrapper if running on the web.

```wat
(import "./dom.js" "write_text" (func $write_text (param externref externref)))
```

---

## 3. Dynamic JS Objects & `externref`

When working with arbitrary external JavaScript values (such as DOM nodes, browser APIs, or custom JS objects):
* They are mapped to a custom Gleam type (like a phantom type) which compiles to `externref` in Wasm.
* To pass internal Wasm GC types to dynamic JS calls, the compiler utilizes the conversion instructions:
  * `extern.convert_any`: Converts a Wasm `anyref` to `externref` so JS can reference it.
  * `any.convert_extern`: Converts an `externref` back to `anyref` when receiving an object from JS.
