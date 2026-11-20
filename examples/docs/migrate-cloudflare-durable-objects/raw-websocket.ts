import { actor } from "rivetkit";

// docs:start actor
// Keep your own WebSocket protocol with onWebSocket
export const echo = actor({
	options: {
		// Keep sockets open when the actor sleeps instead of closing them (experimental)
		canHibernateWebSocket: true,
	},
	onWebSocket: (c, websocket) => {
		websocket.addEventListener("message", (event) => {
			websocket.send(`${c.key[0]}: ${String(event.data)}`);
		});
	},
	actions: {},
});
// docs:end actor
