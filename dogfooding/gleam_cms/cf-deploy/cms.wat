(module
  (type (func (param i32) (result i32)))
  (type (func (param i64) (result i32)))
  (type (func (param i32) (result i64)))
  (type (func (param i32 i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (param i32) (result i32)))
  (type (func (result i64)))
  (type (func (param i64) (result i64)))
  (type (func (param i64 i64) (result i64)))
  (type (func (param i64) (result i64)))
  (memory 1 256)
  (export "memory" (memory 0))
  (global (mut i32)
    i32.const 8 )
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
  (func $box_int (type 1)
    (param $raw i64)
    (local $ptr i32)
    i32.const 16
    call $0
    local.tee 1
    local.get 0
    i64.store offset=0 align=3
    local.get 1
  )
  (func $unbox_int (type 2)
    (param $ptr i32)
    local.get 0
    i64.load offset=0 align=3
  )
  (func $make_tagged (type 3)
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
  (func $get_tag (type 4)
    (param $ptr i32)
    local.get 0
    i32.load offset=0 align=2
  )
  (func $get_payload (type 5)
    (param $ptr i32)
    local.get 0
    i32.load offset=4 align=2
  )
  (func init (type 6)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    i32.const 0
    call $1
  )
  (func add_page (type 7)
    (param count i64)
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
    call $1
  )
  (func find_page (type 8)
    (param count i64)
    (param id i64)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
    local.get 0
    local.set 3
    local.set 2
    local.get 2
    local.get 3
    i32.lt_s
    call $4
    i32.const 2
    i32.eq
    if
then $then
  local.get 0
end
else $else
  i32.const -1
end
end

    call $1
  )
  (func published_count (type 9)
    (param count i64)
    (local $tmp0 i32)
    (local $tmp1 i32)
    (local $tmp2 i32)
    (local $tmp3 i32)
    (local $ftmp0 f64)
    (local $ftmp1 f64)
    local.get 0
    call $1
  )
  (export "init" (func 6))
  (export "add_page" (func 7))
  (export "find_page" (func 8))
  (export "published_count" (func 9))
)
