import { actor } from "rivetkit";

export const echoAgent = actor({
	options: {
		// Keep the socket open when the actor sleeps instead of closing it
		// (experimental, off by default)
		canHibernateWebSocket: true,
	},

	// Handle raw frames with your own protocol, like onMessage
	onWebSocket: (_c, websocket) => {
		websocket.addEventListener("message", (event) => {
			websocket.send(`echo: ${event.data}`);
		});
	},

	actions: {},
});
