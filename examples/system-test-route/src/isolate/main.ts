import type { ActorContext } from "@rivet-gg/actor-core";
import { upgradeWebSocket } from "hono/deno";
import { createAndStartServer } from "../shared/server.js";

// Start server
export default {
	async start(ctx: ActorContext) {
		console.log("Isolate starting");

		// Create and start server with Deno WebSocket upgrader
		console.log("Starting HTTP server");
		const { app, port } = createAndStartServer(() => upgradeWebSocket, ctx.metadata.actor.id);

		const server = Deno.serve(
			{
				port,
			},
			app.fetch,
		);
		await server.finished;
	},
};
