import { actor, setup, type UniversalWebSocket } from "rivetkit";
import { createClient } from "rivetkit/client";

// docs:start server
// Keeps an existing PartyKit message protocol on a raw WebSocket
export const echoRoom = actor({
	// Open sockets, keyed by an ID assigned on connect
	createVars: () => ({ sockets: new Map<string, UniversalWebSocket>() }),

	onWebSocket: (c, websocket) => {
		const id = crypto.randomUUID();
		c.vars.sockets.set(id, websocket);

		// onConnect: the socket is already open here
		websocket.send(`Welcome to room ${c.key[0]}`);

		// onMessage: strings and binary frames arrive as-is
		websocket.addEventListener("message", (event) => {
			for (const [otherId, other] of c.vars.sockets) {
				// room.broadcast(message, [sender.id])
				if (otherId !== id) other.send(event.data);
			}
		});

		// onClose
		websocket.addEventListener("close", () => {
			c.vars.sockets.delete(id);
		});
	},

	actions: {},
});
// docs:end server

export const registry = setup({ use: { echoRoom } });

// docs:start client
const client = createClient<typeof registry>("http://localhost:6420");

// Returns a standard WebSocket, like new PartySocket(...) did
const ws = await client.echoRoom.getOrCreate(["general"]).webSocket();
ws.addEventListener("message", (event: MessageEvent) =>
	console.log(event.data),
);
ws.send("hello");
// docs:end client
