import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { logger } from "hono/logger";
import { streamSSE } from "hono/streaming";
import { deployWithRivetCloud } from "./deploy-with-rivet-cloud.ts";
import { deployWithRivetSelfHosted } from "./deploy-with-rivet-self-hosted.ts";
import type { DeployRequest, LogCallback } from "./utils.ts";

const app = new Hono();

app.use(logger());

app.post("/api/deploy", async (c) => {
	const body = await c.req.json<DeployRequest>();

	return streamSSE(c, async (stream) => {
		const log: LogCallback = async (message: string) => {
			await stream.writeSSE({ data: message, event: "log" });
		};

		try {
			const isCloud = "cloud" in body.kind;

			let result;
			if (isCloud) {
				result = await deployWithRivetCloud(body, log);
			} else {
				result = await deployWithRivetSelfHosted(body, log);
			}

			await stream.writeSSE({
				data: JSON.stringify(result),
				event: "result",
			});
		} catch (error) {
			console.error("Deployment error:", error);
			const errorMessage =
				error instanceof Error ? error.message : String(error);

			await stream.writeSSE({
				data: errorMessage,
				event: "error",
			});
		}
	});
});

// Global error handlers
process.on("unhandledRejection", (reason, promise) => {
	console.error("Unhandled Rejection at:", promise, "reason:", reason);
});

process.on("uncaughtException", (error) => {
	console.error("Uncaught Exception:", error);
	process.exit(1);
});

const PORT = Number(process.env.PORT) || 3001;
serve({
	fetch: app.fetch,
	port: PORT,
});
