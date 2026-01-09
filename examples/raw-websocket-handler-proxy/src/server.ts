import { createNodeWebSocket } from "@hono/node-ws";
import type { Context } from "hono";
import { Hono } from "hono";
import type { WSContext } from "hono/ws";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

registry.startRunner();
const client = createClient<typeof registry>();

const app = new Hono();
const { upgradeWebSocket } = createNodeWebSocket({ app });

// Forward WebSocket connections to actor's WebSocket handler
app.get(
	"/ws/:name",
	upgradeWebSocket(async (c: Context) => {
		const name = c.req.param("name");

		// Connect to actor WebSocket
		const actor = client.chatRoom.getOrCreate(name);
		const actorWs = await actor.websocket("/");

		return {
			onOpen: async (_evt: Event, ws: WSContext) => {
				// Bridge actor WebSocket to client WebSocket
				actorWs.addEventListener("message", (event: MessageEvent) => {
					ws.send(event.data);
				});

				actorWs.addEventListener("close", () => {
					ws.close();
				});
			},
			onMessage: (evt: MessageEvent) => {
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

export default app;
