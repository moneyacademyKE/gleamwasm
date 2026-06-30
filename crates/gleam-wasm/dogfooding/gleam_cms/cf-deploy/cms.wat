(module
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (memory 1 256)
  (export "memory" (memory 0))
  (global (mut i32)
    i32.const 8 )
  (global (mut i32)
    i32.const 0 )
  (global (mut i32)
    i32.const 0 )
  (func $alloc (type 0)
    (param $size i32)
    (local $ptr i32)
    (local $head i32)
    block
  local.get 0
  i32.const 8
  i32.ne
  br_if 0
  global.get 1
  local.tee 2
  i32.const 0
  i32.eq
  br_if 0
  local.get 2
  i32.load offset=0 align=2
  global.set 1
  local.get 2
  return
end
    block
  local.get 0
  i32.const 12
  i32.ne
  br_if 0
  global.get 2
  local.tee 2
  i32.const 0
  i32.eq
  br_if 0
  local.get 2
  i32.load offset=0 align=2
  global.set 2
  local.get 2
  return
end
    global.get 0
    local.set 1
    global.get 0
    local.get 0
    i32.add
    global.set 0
    local.get 1
  )
  (func $free (type 1)
    (param $ptr i32)
    (param $size i32)
    local.get 1
    i32.const 8
    i32.eq
    if
then
  local.get 0
  global.get 1
  i32.store offset=0 align=2
  local.get 0
  global.set 1
end
end

    local.get 1
    i32.const 12
    i32.eq
    if
then
  local.get 0
  global.get 2
  i32.store offset=0 align=2
  local.get 0
  global.set 2
end
end

  )
  (func $make_tagged (type 2)
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
  (func $get_tag (type 3)
    (param $ptr i32)
    local.get 0
    i32.load offset=0 align=2
  )
  (func $get_payload (type 4)
    (param $ptr i32)
    local.get 0
    i32.load offset=4 align=2
  )
  (func $nil (type 5)
    i32.const 3
    i32.const 0
    call $2
  )
  (func $cons (type 6)
    (param $head i32)
    (param $tail i32)
    (local $ptr i32)
    i32.const 12
    call $0
    local.tee 2
    i32.const 4
    i32.store offset=0 align=2
    local.get 2
    local.get 0
    i32.store offset=4 align=2
    local.get 2
    local.get 1
    i32.store offset=8 align=2
    local.get 2
  )
  (func $list_get_head (type 7)
    (param $ptr i32)
    local.get 0
    i32.load offset=4 align=2
  )
  (func $list_get_tail (type 8)
    (param $ptr i32)
    local.get 0
    i32.load offset=8 align=2
  )
  (func init (type 9)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    i32.const 0
  )
  (func add_page (type 10)
    (param count i32)
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
  (func find_page (type 11)
    (param count i32)
    (param id i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 1
    local.get 0
    local.set 3
    local.set 2
    local.get 2
    local.get 3
    i32.lt_s
    i32.const 0
    i32.ne
    if (result i32)
then $then (result i32)
  local.get 1
end
else $else (result i32)
  i32.const -1
end
end

  )
  (func published_count (type 12)
    (param count i32)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
  )
  (export "init" (func 9))
  (export "add_page" (func 10))
  (export "find_page" (func 11))
  (export "published_count" (func 12))
)
