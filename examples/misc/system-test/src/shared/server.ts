import { Hono, type Context } from "hono";
import { serve } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";

export async function createAndStartServer(
	getForwardedFor: (c: Context) => string | null | undefined = (c) =>
		c.req?.header("x-forwarded-for"),
	exitFn: () => void = () => process.exit(1),
) {
	// Setup auto-exit timer
	setTimeout(() => {
		console.error(
			"Actor should've been destroyed by now. Automatically exiting.",
		);
		exitFn();
	}, 60 * 1000);

	// Get port from environment
	const portEnv = process.env.PORT_HTTP;
	if (!portEnv) {
		throw new Error("missing PORT_HTTP");
	}
	const port = Number.parseInt(portEnv);

	// Create app with health endpoint
	const app = new Hono();
	app.get("/health", (c) => c.text("ok"));

	// Create WebSocket handler
	const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({ app });

	// Add WebSocket endpoint with handler
	app.get(
		"/ws",
		upgradeWebSocket((c: Context) => {
			return {
				onOpen(_, ws) {
					ws.send(
						JSON.stringify([
							"init",
							{
								forwardedFor: getForwardedFor(c),
							},
						]),
					);
				},
				onMessage(message, ws) {
					const data =
						typeof message.data === "string"
							? message.data
							: (message as unknown as string);

					if (typeof data === "string") {
						const [eventType, eventData] = JSON.parse(
							data.slice(0, 2 ** 13),
						);
						switch (eventType) {
							case "ping":
								ws.send(JSON.stringify(["pong", eventData]));
								break;
							default:
								console.warn("unknown event", eventType);
								break;
						}
					}
				},
				onClose(event) {
					console.log(
						`WebSocket closed: ${event.code} ${event.reason}`,
					);
				},
				onError(event) {
					console.error("WebSocket error:", event);
				},
			};
		}),
	);

	// Start server
	console.log(`Listening on port ${port}`);
	const server = serve({
		fetch: app.fetch,
		port,
	});

	// Inject WebSocket handler
	injectWebSocket(server);

	return server;
}

