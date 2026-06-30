(module
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32 i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32 i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32 i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (result i64)))
  (type (func (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (result i32)))
  (type (func (param i32) (result i32)))
  (import "gleamunison" "hash_bytes" (func (type 0)))
  (import "gleamunison" "hex_to_bytes" (func (type 1)))
  (import "gleamunison" "hash_equal" (func (type 2)))
  (import "gleamunison" "hash_to_hex" (func (type 3)))
  (import "gleamunison" "state_get" (func (type 4)))
  (import "gleamunison" "state_set" (func (type 5)))
  (import "gleamunison" "file_read" (func (type 6)))
  (import "gleamunison" "file_write" (func (type 7)))
  (import "gleamunison" "log" (func (type 8)))
  (import "gleamunison" "now_ms" (func (type 9)))
  (import "gleamunison" "timestamp" (func (type 10)))
  (import "gleamunison" "eval" (func (type 11)))
  (memory 1 256)
  (export "memory" (memory 0))
  (global (mut i32)
    i32.const 8 )
  (func $alloc (type 12)
    (param $size i32)
    (local $ptr i32)
    global.get 0
    local.tee 1
    local.get 0
    i32.add
    global.set 0
    local.get 1
  )
  (func $make_tagged (type 13)
    (param $tag i32)
    (param $payload i32)
    (local $ret i32)
    i32.const 8
    call $12
    local.tee 2
    local.get 0
    i32.store offset=0 align=2
    local.get 2
    local.get 1
    i32.store offset=4 align=2
    local.get 2
  )
  (func $get_tag (type 14)
    (param $ptr i32)
    local.get 0
    i32.load offset=0 align=2
  )
  (func $get_payload (type 15)
    (param $ptr i32)
    local.get 0
    i32.load offset=4 align=2
  )
  (func $hash_bytes (type 16)
    (param $ptr i32)
    (param $len i32)
    local.get 0
    local.get 1
    call $0
  )
  (func $hex_to_bytes (type 17)
    (param $ptr i32)
    (param $len i32)
    local.get 0
    local.get 1
    call $1
  )
  (func local_var_index (type 18)
    (param lv i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
  )
  (func range (type 19)
    (param start i32)
    (param end i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
    local.get 1
    local.set 3
    local.set 2
    local.get 2
    local.get 3
    i32.gt_s
    i32.const 0
    i32.ne
    if (result i32)
then $then (result i32)
  i32.const 0
end
else $else (result i32)
  local.get 0
end
end

  )
  (func hash (type 20)
    (param n i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
    i32.const 16777619
    local.set 2
    local.set 1
    local.get 1
    local.get 2
    i32.mul
  )
  (func level1 (type 21)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    i32.const 1
    i32.const 2
    local.set 1
    local.set 0
    local.get 0
    local.get 1
    i32.lt_s
    i32.const 0
    i32.ne
    if (result i32)
then $then (result i32)
  i32.const 100
end
else $else (result i32)
  i32.const 0
end
end

  )
  (func state_demo (type 22)
    (param val i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
    i32.const 1
    local.set 2
    local.set 1
    local.get 1
    local.get 2
    i32.add
  )
  (export "local_var_index" (func 18))
  (export "range" (func 19))
  (export "hash" (func 20))
  (export "level1" (func 21))
  (export "state_demo" (func 22))
)
