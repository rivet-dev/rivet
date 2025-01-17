import type { ActorContext } from "@rivet-gg/actor-core";
import { assertEquals, assertExists } from "@std/assert";
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
		// Automatically exit after 1 minute in order to prevent accidental spam
		setTimeout(() => {
			console.error(
				"Actor should've been destroyed by now. Automatically exiting.",
			);
			Deno.exit(1);
		}, 60 * 1000);

		// Find port
		const portEnv = Deno.env.get("PORT_HTTP");
		assertExists(portEnv, "missing PORT_HTTP");
		const port = Number.parseInt(portEnv);

		// Test KV
		console.time("kv-test");
		await ctx.kv.put(["foo", "bar"], 1);
		assertEquals(await ctx.kv.get(["foo", "bar"]), 1, "kv get");
		await ctx.kv.delete(["foo", "bar"]);

		await ctx.kv.putBatch(
			new Map([
				[["batch", "a"], 2],
				[["batch", "b"], 3],
			]),
		);
		const getBatch = await ctx.kv.getBatch([
			["batch", "a"],
			["batch", "b"],
		]);
		assertEquals(getBatch.get(["batch", "a"]), 2, "kv get batch");
		assertEquals(getBatch.get(["batch", "b"]), 3, "kv get batch");

		const list = await ctx.kv.list({
			prefix: ["batch"],
		});
		assertEquals(
			list.array(),
			[
				[["batch", "a"], 2],
				[["batch", "b"], 3],
			],
			"kv list",
		);

		await ctx.kv.deleteBatch([
			["batch", "a"],
			["batch", "b"],
		]);
		assertEquals(await ctx.kv.get(["batch", "a"]), null, "kv get deleted");
		console.timeEnd("kv-test");

		// Start server
		console.log(`Listening on port ${port}`);
		const server = Deno.serve({ port }, app.fetch);
		await server.finished;
	},
};
