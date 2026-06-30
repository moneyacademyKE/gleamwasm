;; wasmtest fixture: linear memory store/load
;; Tests: i32.store, i32.load, memory, alloc pattern
(module $wasmtest_memory
  (memory (export "memory") 1)
  (func $write_and_read (export "write_and_read") (param $addr i32) (param $val i32) (result i32)
    local.get $addr
    local.get $val
    i32.store offset=0 align=2
    local.get $addr
    i32.load offset=0 align=2
  )
)
