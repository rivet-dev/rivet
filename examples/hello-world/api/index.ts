import { Hono } from "hono";

import { registry } from "./actors.ts";

const app = new Hono();

// API routes
app.all("/rivet/*", (c) => registry.handler(c.req.raw));
// app.get("/api", (c) => c.json({ message: "Hello, World!" }));

export default app;
