# Supported Wasm instruction matrix

<!-- GENERATED FILE — do not edit by hand. Regenerate with: `bb scripts/gen_supported_spec.clj` -->

Generated from the `Instr` enum (`crates/gleam-wasm/src/ir/types.rs`) and the binary encoder's opcode table (`crates/gleam-wasm/src/emit/wasm/encoder.rs`, `encode_instr`).

**76 of 96** `Instr` variants reach spec bytes in the binary encoder;
the rest fall through the `_ =>` arm and encode as `unreachable` (0x00). The WAT text backend (`crates/gleam-wasm/src/ir/function.rs` Display) is separate; this matrix is authoritative for `.wasm` output only.

Opcode column: leading bytes of the encoding. `0xFB`/`0xFC` prefixes show
 their subopcode; operands (LEB128 indices/immediates, memarg align+offset)
 follow on the wire and are not listed.

| # | `Instr` variant | mnemonic | opcode |
|---|-----------------|----------|--------|
| 1 | `LocalGet` | `local.get` | `0x20` |
| 2 | `LocalSet` | `local.set` | `0x21` |
| 3 | `LocalTee` | `local.tee` | `0x22` |
| 4 | `I32Const` | `i32.const` | `0x41` |
| 5 | `I64Const` | `i64.const` | `0x42` |
| 6 | `F64Const` | `f64.const` | `0x44` |
| 7 | `RefNull` | `ref.null` | `0xD0 0x71` |
| 8 | `RefI31` | `ref.i31` | ⛔ unreachable fallback |
| 9 | `I32Add` | `i32.add` | `0x6A` |
| 10 | `I32Sub` | `i32.sub` | `0x6B` |
| 11 | `I32Mul` | `i32.mul` | `0x6C` |
| 12 | `I32DivS` | `i32.div_s` | `0x6D` |
| 13 | `I32RemS` | `i32.rem_s` | ⛔ unreachable fallback |
| 14 | `I32Eq` | `i32.eq` | `0x46` |
| 15 | `I32Ne` | `i32.ne` | `0x47` |
| 16 | `I32LtS` | `i32.lt_s` | `0x48` |
| 17 | `I32GtS` | `i32.gt_s` | `0x4A` |
| 18 | `I32LeS` | `i32.le_s` | `0x4C` |
| 19 | `I32GeS` | `i32.ge_s` | `0x4E` |
| 20 | `I64Add` | `i64.add` | `0x7C` |
| 21 | `I64Sub` | `i64.sub` | `0x7D` |
| 22 | `I64Mul` | `i64.mul` | `0x7E` |
| 23 | `I64DivS` | `i64.div_s` | `0x7F` |
| 24 | `I64RemS` | `i64.rem_s` | ⛔ unreachable fallback |
| 25 | `I64Eq` | `i64.eq` | `0x51` |
| 26 | `I64Ne` | `i64.ne` | `0x52` |
| 27 | `I64LtS` | `i64.lt_s` | `0x53` |
| 28 | `I64GtS` | `i64.gt_s` | `0x55` |
| 29 | `I64LeS` | `i64.le_s` | `0x57` |
| 30 | `I64GeS` | `i64.ge_s` | `0x59` |
| 31 | `F64Add` | `f64.add` | `0xA0` |
| 32 | `F64Sub` | `f64.sub` | `0xA1` |
| 33 | `F64Mul` | `f64.mul` | `0xA2` |
| 34 | `F64Div` | `f64.div` | `0xA3` |
| 35 | `F64Eq` | `f64.eq` | `0x61` |
| 36 | `F64Ne` | `f64.ne` | `0x62` |
| 37 | `F64Lt` | `f64.lt` | `0x63` |
| 38 | `F64Gt` | `f64.gt` | `0x64` |
| 39 | `F64Le` | `f64.le` | `0x65` |
| 40 | `F64Ge` | `f64.ge` | `0x66` |
| 41 | `StructNew` | `struct.new` | `0xFB 0x00` |
| 42 | `StructGet` | `struct.get` | `0xFB 0x02` |
| 43 | `StructSet` | `struct.set` | ⛔ unreachable fallback |
| 44 | `ArrayNew` | `array.new` | ⛔ unreachable fallback |
| 45 | `ArrayGet` | `array.get` | ⛔ unreachable fallback |
| 46 | `ArraySet` | `array.set` | ⛔ unreachable fallback |
| 47 | `ArrayLen` | `array.len` | ⛔ unreachable fallback |
| 48 | `RefTest` | `ref.test` | ⛔ unreachable fallback |
| 49 | `RefCast` | `ref.cast` | ⛔ unreachable fallback |
| 50 | `BrOnCast` | `br_on_cast` | ⛔ unreachable fallback |
| 51 | `BrOnCastFail` | `br_on_cast_fail` | ⛔ unreachable fallback |
| 52 | `Block` | `block` | `0x02` |
| 53 | `Loop` | `loop` | ⛔ unreachable fallback |
| 54 | `If` | `if` | `0x04` |
| 55 | `Br` | `br` | `0x0C` |
| 56 | `BrIf` | `br_if` | `0x0D` |
| 57 | `BrTable` | `br_table` | ⛔ unreachable fallback |
| 58 | `Return` | `return` | `0x0F` |
| 59 | `Unreachable` | `unreachable` | `0x00` |
| 60 | `Call` | `call` | `0x10` |
| 61 | `CallRef` | `call_ref` | ⛔ unreachable fallback |
| 62 | `CallIndirect` | `call_indirect` | `0x11` |
| 63 | `ReturnCall` | `return_call` | `0x12` |
| 64 | `ReturnCallRef` | `return_call_ref` | ⛔ unreachable fallback |
| 65 | `RefFunc` | `ref.func` | `0xD2` |
| 66 | `Throw` | `throw_ref` | `0xFB 0x08` |
| 67 | `ExternConvertAny` | `extern.convert_any` | ⛔ unreachable fallback |
| 68 | `AnyConvertExtern` | `any.convert_extern` | ⛔ unreachable fallback |
| 69 | `Drop` | `drop` | `0x1A` |
| 70 | `Select` | `select` | `0x1B` |
| 71 | `GlobalGet` | `global.get` | `0x23` |
| 72 | `GlobalSet` | `global.set` | `0x24` |
| 73 | `I64ExtendI32U` | `i64.extend_i32_u` | `0xAD` |
| 74 | `I32WrapI64` | `i32.wrap_i64` | `0xA7` |
| 75 | `I64Shl` | `i64.shl` | `0x86` |
| 76 | `I64ShrU` | `i64.shr_u` | ⛔ unreachable fallback |
| 77 | `I64Or` | `i64.or` | ⛔ unreachable fallback |
| 78 | `I32And` | `i32.and` | `0x71` |
| 79 | `I32Or` | `i32.or` | `0x72` |
| 80 | `I32Xor` | `i32.xor` | `0x73` |
| 81 | `I32Shl` | `i32.shl` | `0x74` |
| 82 | `I32ShrS` | `i32.shr_s` | `0x75` |
| 83 | `I32ShrU` | `i32.shr_u` | `0x76` |
| 84 | `I32Load` | `i32.load` | `0x28` |
| 85 | `I32Load8U` | `i32.load8_u` | `0x2D` |
| 86 | `I32Store` | `i32.store` | `0x36` |
| 87 | `I32Store8` | `i32.store8` | `0x3A` |
| 88 | `I64Load` | `i64.load` | `0x29` |
| 89 | `I64Store` | `i64.store` | `0x37` |
| 90 | `F64Load` | `f64.load` | `0x2B` |
| 91 | `F64Store` | `f64.store` | `0x39` |
| 92 | `MemoryGrow` | `memory.grow` | `0x40 0x00` |
| 93 | `MemorySize` | `memory.size` | `0x3F 0x00` |
| 94 | `F64ConvertSI32` | `f64.convert_i32_s` | `0xB7` |
| 95 | `F64Trunc` | `f64.trunc` | `0x9D` |
| 96 | `I32TruncSatF64S` | `i32.trunc_sat_f64_s` | `0xFC 0x04` |
