import { createServer } from "node:http";
import { setup } from "rivetkit";
import { deployWithRivetCloud } from "./deploy-with-rivet-cloud.ts";
import { deployWithRivetSelfHosted } from "./deploy-with-rivet-self-hosted.ts";
import type { DeployRequest, LogCallback } from "./utils.ts";

export const registry = setup({});

registry.start();

// Start a separate HTTP server for the deploy API endpoint.
const API_PORT = 6421;

const server = createServer(async (req, res) => {
	// CORS headers
	res.setHeader("Access-Control-Allow-Origin", "*");
	res.setHeader("Access-Control-Allow-Methods", "POST, OPTIONS");
	res.setHeader("Access-Control-Allow-Headers", "Content-Type");

	if (req.method === "OPTIONS") {
		res.writeHead(204);
		res.end();
		return;
	}

	if (req.method === "POST" && req.url === "/api/deploy") {
		// Read body
		const chunks: Buffer[] = [];
		for await (const chunk of req) {
			chunks.push(chunk as Buffer);
		}
		const body: DeployRequest = JSON.parse(Buffer.concat(chunks).toString());

		// Set up SSE response
		res.writeHead(200, {
			"Content-Type": "text/event-stream",
			"Cache-Control": "no-cache",
			Connection: "keep-alive",
		});

		const log: LogCallback = async (message: string) => {
			res.write(`event: log\ndata: ${message}\n\n`);
		};

		try {
			const isCloud = "cloud" in body.kind;

			let result;
			if (isCloud) {
				result = await deployWithRivetCloud(body, log);
			} else {
				result = await deployWithRivetSelfHosted(body, log);
			}

			res.write(`event: result\ndata: ${JSON.stringify(result)}\n\n`);
		} catch (error) {
			console.error("Deployment error:", error);
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			res.write(`event: error\ndata: ${errorMessage}\n\n`);
		}

		res.end();
		return;
	}

	res.writeHead(404);
	res.end("Not found");
});

server.listen(API_PORT, () => {
	console.log(`Deploy API server listening on port ${API_PORT}`);
});

// Global error handlers
process.on("unhandledRejection", (reason, promise) => {
	console.error("Unhandled Rejection at:", promise, "reason:", reason);
});

process.on("uncaughtException", (error) => {
	console.error("Uncaught Exception:", error);
	process.exit(1);
});
