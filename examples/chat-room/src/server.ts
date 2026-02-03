import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();
app.all("/api/rivet/*", async (c) => {
	// Strip /api/rivet prefix since basePath is set to "/"
	const url = new URL(c.req.raw.url);
	const strippedPath = url.pathname.replace(/^\/api\/rivet/, "");
	url.pathname = strippedPath || "/";
	const modifiedRequest = new Request(url.toString(), c.req.raw);
	return registry.handler(modifiedRequest);
});
export default app;
