import { actor } from "rivetkit";

const apiActor = actor({
  state: { requestCount: 0 },

  onRequest: (c, request) => {
    const url = new URL(request.url);
    c.state.requestCount++;

    if (url.pathname === "/api/status") {
      return new Response(JSON.stringify({
        status: "ok",
        requestCount: c.state.requestCount
      }), {
        headers: { "Content-Type": "application/json" }
      });
    }

    return new Response("Not found", { status: 404 });
  },

  actions: { /* ... */ }
});
