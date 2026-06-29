# Gleam-to-Wasm GC Compiler Spec: Type Mapping

This document specifies how Gleam's static type system maps to the native primitives of WebAssembly Garbage Collection (Wasm GC).

---

## 1. Primitive Mappings

To optimize performance and minimize allocations, primitives are represented in their unboxed forms in local variables/parameters, and boxed only when passed as generic (`anyref`) types.

| Gleam Type | Unboxed Wasm GC Type | Boxed Representation | Notes |
| :--- | :--- | :--- | :--- |
| **Int** | `i64` | `(struct (field i64))` | 64-bit signed integers. Can also use `i31ref` for values fits in 31-bit. |
| **Float** | `f64` | `(struct (field f64))` | Double-precision floating point. |
| **Bool** | `i32` (0 or 1) | `i31ref` (0 or 1) | Represented as immediate unboxed references. |
| **Nil** | `nullref` | `nullref` | Uses the native null reference. |

---

## 2. Custom Types & Tagged Unions (ADTs)

Gleam represents custom types as algebraic data types (ADTs) with one or more variants. We translate this to Wasm GC struct subtyping.

### Example Gleam Code:
```gleam
pub type Shape {
  Circle(radius: Float)
  Rect(width: Float, height: Float)
}
```

### Generated Wasm GC (WAT):
```wat
;; Base type for the custom type Shape
(type $Shape (sub (struct)))

;; Variant Circle extends Shape
(type $Circle (sub $Shape (struct 
  (field $radius f64)
)))

;; Variant Rect extends Shape
(type $Rect (sub $Shape (struct 
  (field $width f64)
  (field $height f64)
)))
```

### Pattern Matching / Type Casting:
Instead of generating tag comparisons, the compiler uses Wasm GC's type check and casting instructions:
* Use `ref.test $Circle` to check if a `$Shape` reference is a `$Circle`.
* Use `br_on_cast $label $Shape $Circle` to branch if the type cast succeeds.
This leverages the host VM's native type verification for optimal execution speed.

---

## 3. Tuples & Records

Tuples and records are compiled to flat structural types.

* **Tuples:** A tuple like `#(a, b)` maps to:
  ```wat
  (type $Tuple2 (struct (field $a anyref) (field $b anyref)))
  ```
* **Records:** If a custom type has labeled fields, it compiles to a Wasm struct with named fields for debugging clarity.

---

## 4. Functions & Closures

Gleam functions are first-class and can capture variables from their outer scope (closures).

We represent a closure as a structure containing a pointer to the function code and a reference to its environment:

```wat
;; Definition of the closure structure
(type $Closure (struct
  (field $code (ref $FuncSig))
  (field $env anyref)
))

;; Signature for the internal function
;; Takes the environment as its first parameter, followed by user parameters
(type $FuncSig (sub (func (param anyref) (param anyref) (result anyref))))
```

When invoking a closure:
1. Load the `$Closure` reference from the stack.
2. Extract the `$code` function reference and the `$env` environment reference.
3. Call the function using `call_ref`, passing `$env` as the first argument.
