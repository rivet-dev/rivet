import type { ActorContext } from "@rivet-gg/actor-core";
import { Hono } from "hono";

// Setup Hono app
const app = new Hono();

app.get("/health", (c) => {
	return c.text("ok");
});

app.get("/", (c) => {
	return c.text("hello, world!");
});

// Start server
export default {
	async start(ctx: ActorContext) {
		// Find port
		const portEnv = Deno.env.get("PORT_HTTP");
		if (!portEnv) {
			throw new Error("missing PORT_HTTP");
		}
		const port = Number.parseInt(portEnv);

		// Start server
		console.log(`Listening on port ${port}`);
		const server = Deno.serve({ port }, app.fetch);
		await server.finished;
	},
};
