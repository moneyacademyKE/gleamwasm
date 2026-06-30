import wasmSource from "./cms.wasm";

// Map of function names to their expected param counts
// gleamwasm compiled: init/0, add_page/1, find_page/2, published_count/1

export default {
  async fetch(request) {
    const url = new URL(request.url);
    const path = url.pathname;
    const params = url.searchParams;

    try {
      const wasmModule = await WebAssembly.instantiate(wasmSource, {});
      const exports = wasmModule.exports;

      // Route: GET /init → page count
      if (path === "/init") {
        const count = exports.init();
        return json({ ok: true, count });
      }

      // Route: GET /add_page?count=N → count + 1
      if (path === "/add_page") {
        const count = parseInt(params.get("count") || "0", 10);
        const result = exports.add_page(count);
        return json({ ok: true, count, new_count: result });
      }

      // Route: GET /find_page?count=N&id=M → if id < count return id, else -1
      if (path === "/find_page") {
        const count = parseInt(params.get("count") || "0", 10);
        const id = parseInt(params.get("id") || "0", 10);
        const result = exports.find_page(count, id);
        return json({ ok: true, count, id, found_id: result });
      }

      // Route: GET /published_count?count=N
      if (path === "/published_count") {
        const count = parseInt(params.get("count") || "0", 10);
        const result = exports.published_count(count);
        return json({ ok: true, count, published: result });
      }

      // Route: GET / → help
      if (path === "/") {
        return json({
          endpoints: {
            "/init": "Returns initial page count (0)",
            "/add_page?count=N": "Adds a page, returns new count (N+1)",
            "/find_page?count=N&id=M": "Finds page by id (returns id if id<count, else -1)",
            "/published_count?count=N": "Returns count of published pages",
          },
        });
      }

      return json({ ok: false, error: "Unknown endpoint: " + path }, 404);
    } catch (err) {
      return json({ ok: false, error: err.message }, 500);
    }
  },
};

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}
