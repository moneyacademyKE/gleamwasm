# Cloudflare Workers Compatibility Roadmap

Track for deploying gleamwasm-compiled WASM on Cloudflare Workers.

**Status: Fully implemented for linear memory target. Self-contained WASM verified on wrangler.**

## Decision Matrix

| Approach | GC Instructions | Binary Size | Runtime Dependencies | Status |
|----------|:---:|:---:|:---|:---:|
| Direct GC target | Yes (`struct.new`, `ref.test`) | ~94 bytes | Wasm GC engine (Chrome 119+) | ❌ Not on CF |
| Self-contained WASM | No GC ops | ~3KB (17 builtins) | None (pure WASM) | ✅ Verified |
| JS import bridge | No GC ops | ~3KB | JS stubs (12 imports) | ✅ Verified |

## Target Environment Support

| Environment | GC Target | Linear (CF) | Self-contained |
|-------------|:---:|:---:|:---:|
| Cloudflare Workers | ❌ No GC support | ✅ | ✅ |
| Chrome 119+ / V8 12+ | ✅ | ✅ | ✅ |
| Firefox 120+ | ✅ | ✅ | ✅ |
| Safari 17.4+ | ✅ | ✅ | ✅ |
| wasmtime (`--wasm gc=y`) | ✅ | ✅ | ✅ |
| Node.js 22+ (`--experimental-wasm-gc`) | ✅ | ✅ | ✅ |
| Deno 2.0+ | ✅ | ✅ | ✅ |

## Phase 1: Gap Documentation ✅ DONE

- [x] Identified all GC-dependent instructions incompatible with Cloudflare Workers MVP Wasm
- [x] Documented workerd's `compatibility-date.capnp` — zero GC proposal flags
- [x] Analyzed 3 alternative approaches: linear memory with manual GC, JS polyfill, wait for CF to enable GC
- [x] Selected linear memory with tagged values as pragmatic short-term path

## Phase 2: Linear Memory Backend ✅ DONE

### Runtime Layer
- [x] Bump allocator (`$alloc` via global heap pointer)
- [x] Tagged value system (`$make_tagged`, `$get_tag`, `$get_payload`)
- [x] FNV-1a hash (pure WASM, `$hash_bytes`)
- [x] Linear KV store (`$state_get`, `$state_set`)
- [x] Byte-level memcpy (`$memcpy`)
- [x] Memory export + global section

### Codegen
- [x] `--target wasm-cf` CLI flag
- [x] Expression compiler: Int, Float, Bool, Nil, BinOp, If/else, Let, Match, Call, TailCall
- [x] ADT match lowering: GC `ref.test`/`br_on_cast` → `i32.eq` tag comparison
- [x] Param name lookup via `var_map`

### Binary Encoding
- [x] Section ordering: type→import→func→mem→global→export→code
- [x] Signed LEB128 for negative i32
- [x] One-armed If with correct blocktype (0x40)
- [x] Workerd-validated binary output

### Validation
- [x] Pre-emission module validation (`validate_module()`)
- [x] GC instruction detection in linear modules
- [x] Local bound checking
- [x] Call index checking
- [x] Branch depth checking
- [x] Export index checking

## Phase 3: Ship-Unblock When CF Enables Wasm GC 📋 Future

Track Cloudflare Workers GC support:
- [ ] Monitor workerd releases for GC proposal flags
- [ ] Test GC target against workerd when available
- [ ] Migration path: linear → GC target for existing deployments

## Phase 4: Hybrid JS Proxy 📋 Future

- [ ] JS-side heap management (WebAssembly.Memory from JS)
- [ ] Import bridge for file/KV/crypto operations
- [ ] Durable Object persistence layer
- [ ] KV-backed file system
- [ ] Web Crypto API hash/hex operations

## Dogfooded Apps (All Verified on Wrangler)

| App | Target | Size | Imports | Wrangler |
|-----|--------|------|---------|----------|
| Lustre Counter | Linear | 186 bytes | 0 | ✅ |
| BEAM CMS | Linear | 270 bytes | 0 | ✅ |
| GleamUnison CF | Linear | ~3KB | 12 JS | ✅ |
| GleamUnison SC | Linear | ~3KB | 0 | ✅ |

## Quick Deploy

```bash
# Build for Cloudflare
cargo test test_cms_full_deploy --test dogfood_cms -- --nocapture

# Deploy locally
cd dogfooding/gleam_cms/cf-deploy
npx wrangler dev --port 8792

# Test
curl http://localhost:8792/init
curl http://localhost:8792/add_page?count=0
```
