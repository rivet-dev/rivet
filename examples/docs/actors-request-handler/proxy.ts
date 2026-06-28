import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { serve } from "@hono/node-server";

const client = createClient();

const app = new Hono();

// Proxy requests to actor's onRequest handler
app.all("/actors/:id/:path{.*}", async (c) => {
    const actorId = c.req.param("id");
    const actorPath = (c.req.param("path") || "");

    // Rewrite the incoming request to the actor-relative path, preserving
    // method, headers, and body
    const url = new URL(actorPath, "http://actor");
    const actorRequest = new Request(url, c.req.raw);

    // Forward the rewritten Request to the actor's onRequest handler
    const actor = client.counter.get(actorId);
    return await actor.fetch(actorRequest);
});

serve(app);
