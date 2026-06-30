(module
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32) (result i32)))
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
  (type (func (param i32 i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (result i32)))
  (type (func (param i32) (result i32)))
  (memory 2 256)
  (export "memory" (memory 0))
  (global (mut i32)
    i32.const 256 )
  (func $alloc (type 0)
    (param $size i32)
    (local $ptr i32)
    global.get 0
    local.tee 1
    local.get 0
    i32.add
    global.set 0
    local.get 1
  )
  (func $make_tagged (type 1)
    (param $tag i32)
    (param $payload i32)
    (local $ret i32)
    i32.const 8
    call $0
    local.tee 2
    local.get 0
    i32.store offset=0 align=2
    local.get 2
    local.get 1
    i32.store offset=4 align=2
    local.get 2
  )
  (func $get_tag (type 2)
    (param $ptr i32)
    local.get 0
    i32.load offset=0 align=2
  )
  (func $get_payload (type 3)
    (param $ptr i32)
    local.get 0
    i32.load offset=4 align=2
  )
  (func $hash_bytes (type 4)
    (param $data i32)
    (param $len i32)
    (local $hash i32)
    (local $i i32)
    (local $byte i32)
    (local $result_ptr i32)
    i32.const 40389
    i32.const 33052
    i32.const 16
    i32.shl
    i32.or
    local.set 2
    i32.const 0
    local.set 3
    loop $hash_loop
  local.get 0
  local.get 3
  i32.add
  i32.load8_u offset=0 align=0
  local.set 4
  local.get 2
  local.get 4
  i32.xor
  local.set 2
  local.get 2
  i32.const 403
  i32.const 256
  i32.const 16
  i32.shl
  i32.or
  i32.mul
  local.set 2
  local.get 3
  i32.const 1
  i32.add
  local.set 3
  local.get 3
  local.get 1
  i32.lt_s
  br_if 0
end
    i32.const 4
    call $0
    local.tee 5
    local.get 2
    i32.store offset=0 align=2
    local.get 5
  )
  (func $hex_to_bytes (type 5)
    (param $ptr i32)
    (param $len i32)
    local.get 0
  )
  (func $hash_equal (type 6)
    (param $a_ptr i32)
    (param $a_len i32)
    (param $b_ptr i32)
    (param $b_len i32)
    (local $i i32)
    block $check_len
  local.get 1
  local.get 3
  i32.eq
  br_if 0
  i32.const 0
  return
end
    i32.const 0
    local.set 4
    loop $eq_loop
  local.get 0
  local.get 4
  i32.add
  i32.load8_u offset=0 align=0
  local.get 2
  local.get 4
  i32.add
  i32.load8_u offset=0 align=0
  i32.ne
  br_if 1
  local.get 4
  i32.const 1
  i32.add
  local.set 4
  local.get 4
  local.get 1
  i32.lt_s
  br_if 0
end
    i32.const 1
    return
  )
  (func $hash_to_hex (type 7)
    (param $ptr i32)
    (param $len i32)
    local.get 0
  )
  (func $state_get (type 8)
    (param $key_ptr i32)
    (param $key_len i32)
    (local $hash_val i32)
    (local $bucket i32)
    local.get 0
    local.get 1
    call $4
    local.tee 2
    i32.load offset=0 align=2
    local.set 3
    local.get 3
    i32.const 0
    i32.eq
    if (result i32)
then $not_found (result i32)
  i32.const 0
  return
end
end

    local.get 3
  )
  (func $state_set (type 9)
    (param $key_ptr i32)
    (param $key_len i32)
    (param $val_ptr i32)
    (param $val_len i32)
    (local $hash i32)
    (local $alloc_ptr i32)
    local.get 0
    local.get 1
    call $4
    local.set 4
    local.get 3
    call $0
    local.set 5
    local.get 5
    local.get 2
    local.get 3
    call $16
    drop
    local.get 4
    i32.const 128
    i32.add
    local.get 5
    i32.store offset=0 align=2
    i32.const 1
  )
  (func $file_read (type 10)
    (param $path i32)
    (param $len i32)
    i32.const -1
  )
  (func $file_write (type 11)
    (param $path i32)
    (param $path_len i32)
    (param $data i32)
    (param $data_len i32)
    i32.const 1
  )
  (func $log (type 12)
    (param $msg i32)
    (param $len i32)
    i32.const 1
  )
  (func $now_ms (type 13)
    i64.const 0
  )
  (func $timestamp (type 14)
    i32.const 0
  )
  (func $eval (type 15)
    (param $expr i32)
    (param $len i32)
    (local $ptr i32)
    i32.const 8
    call $0
    local.tee 2
    local.get 0
    i32.store offset=0 align=2
    local.get 2
  )
  (func $memcpy (type 16)
    (param $dst i32)
    (param $src i32)
    (param $len i32)
    (local $i i32)
    i32.const 0
    local.set 3
    loop $memcpy_loop
  local.get 0
  local.get 3
  i32.add
  local.get 1
  local.get 3
  i32.add
  i32.load8_u offset=0 align=0
  i32.store8 offset=0 align=0
  local.get 3
  i32.const 1
  i32.add
  local.tee 3
  local.get 2
  i32.lt_s
  br_if 0
end
    local.get 0
  )
  (func local_var_index (type 17)
    (param lv i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
  )
  (func range (type 18)
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
  (func hash (type 19)
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
  (func level1 (type 20)
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
  (func state_demo (type 21)
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
  (export "local_var_index" (func 17))
  (export "range" (func 18))
  (export "hash" (func 19))
  (export "level1" (func 20))
  (export "state_demo" (func 21))
)
