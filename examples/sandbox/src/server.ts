import { Hono } from "hono";
import { registry } from "./index.ts";
import { runBenchmarks } from "./bench-endpoint.ts";

const app = new Hono();

app.get("/api/bench", async (c) => {
	const filter = c.req.query("filter");
	const results = await runBenchmarks(registry, filter);
	return c.json(results);
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
