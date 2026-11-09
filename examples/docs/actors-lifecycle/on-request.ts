import { actor } from "rivetkit";

const apiActor = actor({
  vars: { requestCount: 0 },

  onRequest: (c, request) => {
    const url = new URL(request.url);
    c.vars.requestCount++;

    if (url.pathname === "/api/status") {
      return new Response(JSON.stringify({
        status: "ok",
        requestCount: c.vars.requestCount
      }), {
        headers: { "Content-Type": "application/json" }
      });
    }

    return new Response("Not found", { status: 404 });
  },

  actions: { /* ... */ }
});
