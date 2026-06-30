import wasmSource from "./gleamunison_sc.wasm";

// Self-contained GleamUnison WASM: ZERO JS imports needed.
// All 12 FFI stubs are pure WASM functions.

export default {
  async fetch(request) {
    const url = new URL(request.url);
    const path = url.pathname;
    const params = url.searchParams;

    try {
      const wasmModule = await WebAssembly.instantiate(wasmSource, {});
      const exports = wasmModule.exports;

      if (path === "/") {
        return json({
          gleamunison_sc: "Self-contained GleamUnison WASM — zero JS imports",
          exports: Object.keys(exports).filter(k => typeof exports[k] === "function"),
          endpoints: {
            "/local_var_index?lv=N": "identity",
            "/range?start=N&end=M": "range base case",
            "/hash?n=N": "FNV-like hash",
            "/level1": "integer comparison",
            "/state_demo?val=N": "simulated state mutation",
          },
        });
      }

      if (path === "/local_var_index") {
        return json({ function: "local_var_index", lv: params.get("lv"), result: exports.local_var_index(parseInt(params.get("lv")||"0",10)) });
      }
      if (path === "/range") {
        return json({ function: "range", start: params.get("start"), end: params.get("end"), result: exports.range(parseInt(params.get("start")||"0",10), parseInt(params.get("end")||"0",10)) });
      }
      if (path === "/hash") {
        return json({ function: "hash", n: params.get("n"), result: exports.hash(parseInt(params.get("n")||"0",10)) });
      }
      if (path === "/level1") {
        return json({ function: "level1", result: exports.level1() });
      }
      if (path === "/state_demo") {
        return json({ function: "state_demo", val: params.get("val"), result: exports.state_demo(parseInt(params.get("val")||"0",10)) });
      }

      return json({ error: "Unknown endpoint: " + path }, 404);
    } catch (err) {
      return json({ error: err.message, stack: err.stack }, 500);
    }
  },
};

function json(data, status = 200) {
  return new Response(JSON.stringify(data), { status, headers: { "Content-Type": "application/json" } });
}
