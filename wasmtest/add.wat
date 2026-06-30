;; wasmtest fixture: simple addition
;; This is a manual WAT test that verifies our WAT → binary round-trip
;; and that wasmtime/wrk can execute the result.
(module $wasmtest_add
  (type (func (param i64 i64) (result i64)))
  (func $add (export "add") (type 0) (param $a i64) (param $b i64) (result i64)
    local.get $a
    local.get $b
    i64.add
  )
)
