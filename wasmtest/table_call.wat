;; wasmtest fixture: call_indirect through an elem-populated funcref table (GAP #6)
;; Verifies table/element sections + call_indirect execute correctly under wasmtime.
;; invoke: run_table
;; expect: 42
(module $wasmtest_table_call
  (type $sig_i64_i64 (func (param i64) (result i64)))
  (type $sig_i64 (func (result i64)))
  (table 2 funcref)
  (elem (i32.const 0) $double)
  (func $double (type $sig_i64_i64) (param $x i64) (result i64)
    local.get $x
    i64.const 2
    i64.mul
  )
  (func $run_table (type $sig_i64) (result i64)
    i64.const 21
    i32.const 0
    call_indirect (type $sig_i64_i64)
  )
  (export "run_table" (func $run_table))
)
