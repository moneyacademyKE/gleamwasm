import wasmSource from "./counter_cf.wasm";

export default {
  async fetch(request) {
    const url = new URL(request.url);
    const state = parseInt(url.searchParams.get("state") || "0", 10);

    try {
      const wasmModule = await WebAssembly.instantiate(wasmSource, {});
      const { update, memory } = wasmModule.exports;

      // Heap pointer is a mutable global initialized to 8 — no JS init needed

      const result = update(state);

      return new Response(JSON.stringify({
        ok: true,
        state,
        result,
      }), {
        headers: { "Content-Type": "application/json" },
      });
    } catch (err) {
      return new Response(JSON.stringify({
        ok: false,
        error: err.message,
      }), {
        status: 500,
        headers: { "Content-Type": "application/json" },
      });
    }
  },
};
