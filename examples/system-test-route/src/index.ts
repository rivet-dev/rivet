import { serve } from "@hono/node-server";
import { Hono } from "hono";

// Setup auto-exit timer
setTimeout(() => {
	console.error(
		"Actor should've been destroyed by now. Automatically exiting.",
	);

	if (typeof Deno !== "undefined") Deno.exit(1);
	else process.exit(1);
}, 60 * 1000);

let tickIndex = 0;
setInterval(() => {
	tickIndex++;
	console.log("Tick", tickIndex);
	console.log(JSON.stringify({ level: "info", message: "tick", tickIndex }));
	console.log(`level=info message=tick tickIndex=${tickIndex}`);
}, 1000);

// Get port from environment
const portEnv =
	typeof Deno !== "undefined"
		? Deno.env.get("PORT_HTTP")
		: process.env.PORT_HTTP;
if (!portEnv) {
	throw new Error("missing PORT_HTTP");
}
const port = Number.parseInt(portEnv);

// Create app with health endpoint
const app = new Hono();

app.get("/health", (c) => c.text("ok"));

// Add a catch-all route to handle any other path (for testing routeSubpaths)
app.all("*", (c) => {
	console.log(
		`Received request to ${c.req.url} from ${c.req.header("x-forwarded-for") || "unknown"}`,
	);
	return c.json({
		path: c.req.path,
		query: c.req.query(),
	});
});

console.log(`Listening on port ${port}`);

serve({ fetch: app.fetch, port });
