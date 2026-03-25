import { Hono } from "hono";
import { registry } from "./index.ts";

const app = new Hono();

app.get("/health", (c) => {
	return c.json({ ok: true });
});

app.get("/api/rivet/health", (c) => {
	return c.json({ ok: true });
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
