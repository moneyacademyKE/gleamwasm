(module
  (type (func (result i64)))
  (type (func (param i32) (result i32)))
  (type (func (param i32 i32) (result i64)))
  (type (func (param i64) (result i32)))
  (type (func (param i64) (result i32)))
  (type (func (param i32) (result i64)))
  (type (func (param i64) (result i32)))
  (memory (export "memory") 1
    (max 256)
  )
  (func $alloc (type 1)
    (param $size i32)
    (local $ptr i32)
    i32.const 0
    i32.load offset=0 align=2
    local.tee 1
    local.get 0
    i32.add
    i32.const 0
    i32.store offset=0 align=2
    local.get 1
  )
  (func $make_tagged (type 2)
    (param $tag i32)
    (param $payload i32)
    local.get 0
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get 1
    i64.extend_i32_u
    i64.or
  )
  (func $get_tag (type 3)
    (param $val i64)
    local.get 0
    i64.const 32
    i64.shr_u
    i32.wrap_i64
  )
  (func $get_payload (type 4)
    (param $val i64)
    local.get 0
    i32.wrap_i64
  )
  (func $box_int (type 5)
    (param $n i32)
    (local $ptr i32)
    i32.const 4
    call $0
    local.tee 1
    local.get 0
    i32.store offset=0 align=2
    i32.const 0
    local.get 1
    call $1
  )
  (func $unbox_int (type 6)
    (param $val i64)
    local.get 0
    call $3
    i32.load offset=0 align=2
  )
)
