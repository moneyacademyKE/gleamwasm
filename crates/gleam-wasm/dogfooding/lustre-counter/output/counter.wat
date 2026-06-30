(module
  (type $Int (sub (struct (field $value i64)))
  (type $Float (sub (struct (field $value f64)))
  (type $Action (sub (struct))
  (type $Incr (sub $Action) (struct))
  (type $Decr (sub $Action) (struct))
  (type (func (param i64 (ref null struct)) (result i64)))
  (func update (type 5)
    (param state i64)
    (param action (ref null struct))
    local.get 1
    local.set 0
    block $match_exit
  local.get 0
  ref.test (ref null $1)
  if
then $case_1
  local.get 0
  ref.cast (ref $1)
  local.get 0
  i64.const 1
  i64.add
  br 2
end
end

  local.get 0
  ref.test (ref null $2)
  if
then $case_2
  local.get 0
  ref.cast (ref $2)
  local.get 0
  i64.const 1
  i64.sub
  br 2
end
else $no_match
  unreachable
end
end

end
  )
  (export "update" (func 0))
)
