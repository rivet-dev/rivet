import { registry } from "./index.ts";
import { serve } from "@hono/node-server";
import { Hono } from "hono";

const app = new Hono();

function requestHeaders(headers: Headers) {
	return Object.fromEntries(
		Array.from(headers.entries()).map(([key, value]) => [
			key,
			key === "authorization" || key === "x-rivet-token"
				? "<redacted>"
				: value,
		]),
	);
}

app.use("*", async (c, next) => {
	const startedAt = Date.now();
	await next();
	console.log(
		JSON.stringify({
			kind: "request",
			method: c.req.method,
			path: new URL(c.req.url).pathname,
			headers: requestHeaders(c.req.raw.headers),
			status: c.res.status,
			durationMs: Date.now() - startedAt,
		}),
	);
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
app.all("/api/rivet", (c) => registry.handler(c.req.raw));

serve({ fetch: app.fetch, port: 3000 }, () => {
	console.log(
		"serverless RivetKit listening on http://127.0.0.1:3000/api/rivet",
	);
});
