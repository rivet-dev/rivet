import { serve } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import { Hono } from "hono";
import { registry } from "./registry.js";

const { client } = registry.start({
	cors: {
		origin: "http://localhost:5173",
		credentials: true,
	},
});

const app = new Hono();
const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });

// Forward WebSocket connections to actor's WebSocket handler
app.get(
	"/ws/:name",
	upgradeWebSocket(async (c) => {
		const name = c.req.param("name");

		// Connect to actor WebSocket
		const actor = client.chatRoom.getOrCreate(name);
		const actorWs = await actor.websocket("/");

		return {
			onOpen: async (_evt, ws) => {
				// Bridge actor WebSocket to client WebSocket
				actorWs.addEventListener("message", (event: MessageEvent) => {
					ws.send(event.data);
				});

				actorWs.addEventListener("close", () => {
					ws.close();
				});
			},
			onMessage: (evt) => {
				// Forward message to actor WebSocket
				if (actorWs && typeof evt.data === "string") {
					actorWs.send(evt.data);
				}
			},
			onClose: () => {
				// Forward close to actor WebSocket
				if (actorWs) {
					actorWs.close();
				}
			},
		};
	}),
);

const server = serve({ fetch: app.fetch, port: 8080 });
injectWebSocket(server);
console.log("Listening on port 8080");
