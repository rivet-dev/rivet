import { serve } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import { createAndStartServer } from "../shared/server.js";

let injectWebSocket: any;
const { app, port } = createAndStartServer((app) => {
	// Get Node.js WebSocket handler
	const result = createNodeWebSocket({ app });
	injectWebSocket = result.injectWebSocket;
	return result.upgradeWebSocket;
});

const server = serve({ fetch: app.fetch, port });
injectWebSocket(server);
