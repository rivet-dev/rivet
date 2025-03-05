import type { ActorContext } from "@rivet-gg/actor-core";
import { Hono } from "hono";
import { upgradeWebSocket } from "hono/deno";

// Setup Hono app
const app = new Hono();

app.get("/health", (c) => {
	return c.text("ok");
});

app.get(
	"/ws",
	upgradeWebSocket((c) => {
		return {
			onOpen(_event, ws) {
				ws.send(
					JSON.stringify([
						"init",
						{
							forwardedFor: c.header("x-forwarded-for"),
						},
					]),
				);
			},
			onMessage(event, ws) {
				if (typeof event.data === "string") {
					const [eventType, data] = JSON.parse(
						event.data.slice(0, 2 ** 13),
					);
					switch (eventType) {
						case "ping":
							ws.send(JSON.stringify(["pong", data]));
							break;
						default:
							console.warn("unknown event", eventType);
							break;
					}
				}
			},
		};
	}),
);

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
