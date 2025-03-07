import { createAndStartServer } from "../shared/server.js";
import { createNodeWebSocket } from "@hono/node-ws";
import { serve } from "@hono/node-server";

// Automatically exit after 1 minute in order to prevent accidental spam
setTimeout(() => {
	console.error(
		"Actor should've been destroyed by now. Automatically exiting.",
	);
	process.exit(1);
}, 60 * 1000);

let injectWebSocket: any;
const { app, port } = createAndStartServer(
	(app) => {
		// Get Node.js WebSocket handler
		const result = createNodeWebSocket({ app });
		injectWebSocket = result.injectWebSocket;
		return result.upgradeWebSocket;
	}
);

const server = serve({ fetch: app.fetch, port });
injectWebSocket(server);
