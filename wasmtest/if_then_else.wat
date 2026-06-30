;; wasmtest fixture: i32 arithmetic + control flow
;; Tests: i32.add, i32.mul, if/else, local.set/get
(module $wasmtest_if_then_else
  (func $max (export "max") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.gt_s
    if (result i32)
      local.get $a
    else
      local.get $b
    end
  )
)
