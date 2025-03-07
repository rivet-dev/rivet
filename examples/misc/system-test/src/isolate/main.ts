import type { ActorContext } from "@rivet-gg/actor-core";
import { createAndStartServer } from "../shared/server.js";
import { upgradeWebSocket } from "hono/deno";

// Start server
export default {
	async start(ctx: ActorContext) {
		console.log("Isolate starting");

		// Test KV functionality
		console.log("Starting KV API validation");
		console.time("kv-test");

		console.log("Testing simple put/get/delete operations");
		await ctx.kv.put(["foo", "bar"], 1);
		if ((await ctx.kv.get(["foo", "bar"])) !== 1) {
			throw new Error("kv get failed - value mismatch");
		}
		await ctx.kv.delete(["foo", "bar"]);

		console.log("Testing batch operations");
		await ctx.kv.putBatch(
			new Map([
				[["batch", "a"], 2],
				[["batch", "b"], 3],
			]),
		);

		console.log("Testing getBatch operations");
		const getBatch = await ctx.kv.getBatch([
			["batch", "a"],
			["batch", "b"],
		]);
		if (getBatch.get(["batch", "a"]) !== 2) {
			throw new Error("kv getBatch failed - value mismatch for key a");
		}
		if (getBatch.get(["batch", "b"]) !== 3) {
			throw new Error("kv getBatch failed - value mismatch for key b");
		}

		console.log("Testing list operations");
		const list = await ctx.kv.list({
			prefix: ["batch"],
		});
		if (
			JSON.stringify(list.array()) !==
			JSON.stringify([
				[["batch", "a"], 2],
				[["batch", "b"], 3],
			])
		) {
			throw new Error("kv list failed - results don't match expected values");
		}

		console.log("Testing deleteBatch operations");
		await ctx.kv.deleteBatch([
			["batch", "a"],
			["batch", "b"],
		]);
		if ((await ctx.kv.get(["batch", "a"])) !== null) {
			throw new Error("kv deleteBatch failed - key still exists");
		}

		console.timeEnd("kv-test");
		console.log("KV API validation complete");

		// Create and start server with Deno WebSocket upgrader
		console.log("Starting HTTP/WebSocket server");
		const { app, port } = createAndStartServer(
			() => upgradeWebSocket,
		);

		const server = Deno.serve({
			port
		}, app.fetch);
		await server.finished;
	},
};
