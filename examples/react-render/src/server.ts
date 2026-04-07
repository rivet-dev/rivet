import { Hono } from "hono";
import { serveStatic } from "@hono/node-server/serve-static";
import { registry } from "./actors.ts";

const app = new Hono();

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

app.get("/health", (c) => c.json({ status: "ok" }));

app.use("/*", serveStatic({ root: "./public" }));

app.get("*", serveStatic({ root: "./public", path: "/index.html" }));

export default app;
