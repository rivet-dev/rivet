import { Hono } from "hono";
import { serveStatic } from "@hono/node-server/serve-static";
import { registry } from "./actors.ts";
import { port } from "./env.ts";

const app = new Hono();
const handler = registry.fetchHandler({
	path: "/api/rivet",
	dev: `http://127.0.0.1:${port}/api/rivet`,
});

app.all("/api/rivet/*", (c) => handler(c.req.raw));

app.get("/health", (c) => c.json({ status: "ok" }));

app.use("/*", serveStatic({ root: "./public" }));

app.get("*", serveStatic({ root: "./public", path: "/index.html" }));

export default app;
