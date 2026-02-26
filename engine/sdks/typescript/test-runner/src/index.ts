import { serve } from "@hono/node-server";
import type { ActorConfig, RunnerConfig } from "@rivetkit/engine-runner";
import { Runner } from "@rivetkit/engine-runner";
import { Hono, type Context as HonoContext, type Next } from "hono";
import { streamSSE } from "hono/streaming";
import type { Logger } from "pino";
import type WebSocket from "ws";
import { getLogger } from "./log";

const INTERNAL_SERVER_PORT = process.env.INTERNAL_SERVER_PORT
	? Number(process.env.INTERNAL_SERVER_PORT)
	: 5051;
const RIVET_NAMESPACE = process.env.RIVET_NAMESPACE ?? "default";
const RIVET_RUNNER_NAME = process.env.RIVET_RUNNER_NAME ?? "test-runner";
const RIVET_RUNNER_KEY = process.env.RIVET_RUNNER_KEY;
const RIVET_RUNNER_VERSION = process.env.RIVET_RUNNER_VERSION
	? Number(process.env.RIVET_RUNNER_VERSION)
	: 1;
const RIVET_RUNNER_TOTAL_SLOTS = parseInt(process.env.RIVET_RUNNER_TOTAL_SLOTS ?? "1");
const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";
const AUTOSTART_SERVER = (process.env.AUTOSTART_SERVER ?? "1") == "1";
const AUTOSTART_RUNNER = (process.env.AUTOSTART_RUNNER ?? "0") == "1";
const AUTOCONFIGURE_SERVERLESS = (process.env.AUTOCONFIGURE_SERVERLESS ?? "1") == "1";

const runnerStarted = Promise.withResolvers<Runner>();
const runnerStopped = Promise.withResolvers<Runner>();
let runner: Runner | null = null;
const websocketLastMsgIndexes: Map<string, number> = new Map();

// Create internal server
const app = new Hono();

function loggerMiddleware(logger: Logger) {
	return async (c: HonoContext, next: Next) => {
		const method = c.req.method;
		const path = c.req.path;
		const startTime = Date.now();

		await next();

		const duration = Date.now() - startTime;
		logger.debug({
			msg: "http request",
			method,
			path,
			status: c.res.status,
			dt: `${duration}ms`,
			reqSize: c.req.header("content-length"),
			resSize: c.res.headers.get("content-length"),
			userAgent: c.req.header("user-agent"),
		});
	};
}
app.use("*", loggerMiddleware(getLogger()));

app.get("/wait-ready", async (c) => {
	const runner = await runnerStarted.promise;
	return c.json(runner.runnerId);
});

app.get("/has-actor", async (c) => {
	const actorIdQuery = c.req.query("actor");
	const generationQuery = c.req.query("generation");
	const generation = generationQuery ? Number(generationQuery) : undefined;

	if (!actorIdQuery || !runner?.hasActor(actorIdQuery, generation)) {
		return c.text("", 404);
	}
	return c.text("ok");
});

app.get("/health", (c) => {
	return c.text("ok");
});

app.get("/shutdown", async (c) => {
	await runner?.shutdown(true);
	return c.text("ok");
});

app.get("/api/rivet/start", async (c) => {
	return streamSSE(c, async (stream) => {
		const runnerStarted = Promise.withResolvers<Runner>();
		const runnerStopped = Promise.withResolvers<Runner>();
		const runner = await startRunner(runnerStarted, runnerStopped);

		c.req.raw.signal.addEventListener("abort", () => {
			getLogger().debug("SSE aborted, shutting down runner");
			runner!.shutdown(true);
		});

		await runnerStarted.promise;

		stream.writeSSE({ data: runner.getServerlessInitPacket()! });

		await runnerStopped.promise;
	});
});

app.get("/api/rivet/metadata", async (c) => {
	return c.json({
		// Not actually rivetkit
		runtime: "rivetkit",
		version: "1",
	});
});

if (AUTOSTART_SERVER) {
	serve({
		fetch: app.fetch,
		port: INTERNAL_SERVER_PORT,
	});
	getLogger().info(
		`Internal HTTP server listening on port ${INTERNAL_SERVER_PORT}`,
	);
}

if (AUTOSTART_RUNNER) {
	runner = await startRunner(runnerStarted, runnerStopped);
} else if (AUTOCONFIGURE_SERVERLESS) {
	await autoConfigureServerless();
}

process.on("SIGTERM", async () => {
	getLogger().debug("received SIGTERM, force exiting in 3s");

	await new Promise(res => setTimeout(res, 3000));

	process.exit(0);
});
process.on("SIGINT", async () => {
	getLogger().debug("received SIGTERM, force exiting in 3s");

	await new Promise(res => setTimeout(res, 3000));

	process.exit(0);
});

async function autoConfigureServerless() {
	getLogger().info("Configuring serverless");

	const res = await fetch(
		`${RIVET_ENDPOINT}/runner-configs/${RIVET_RUNNER_NAME}?namespace=${RIVET_NAMESPACE}`,
		{
			method: "PUT",
			headers: {
				Authorization: `Bearer ${RIVET_TOKEN}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				datacenters: {
					default: {
						serverless: {
							url: `http://localhost:${INTERNAL_SERVER_PORT}`,
							max_runners: 10000,
							slots_per_runner: 1,
							request_lifespan: 300,
						},
					},
				},
			}),
		},
	);

	if (!res.ok) {
		throw new Error(
			`request failed: ${res.statusText} (${res.status}):\n${await res.text()}`,
		);
	}
}

async function startRunner(
	runnerStarted: PromiseWithResolvers<Runner>,
	runnerStopped: PromiseWithResolvers<Runner>,
): Promise<Runner> {
	getLogger().info("Starting runner");
	let runner: Runner;
	const config: RunnerConfig = {
		logger: getLogger(),
		version: RIVET_RUNNER_VERSION,
		endpoint: RIVET_ENDPOINT,
		token: RIVET_TOKEN,
		namespace: RIVET_NAMESPACE,
		runnerName: RIVET_RUNNER_NAME,
		runnerKey:
			RIVET_RUNNER_KEY ?? `key-${Math.floor(Math.random() * 10000)}`,
		totalSlots: RIVET_RUNNER_TOTAL_SLOTS,
		prepopulateActorNames: {},
		onConnected: () => {
			runnerStarted.resolve(runner);
		},
		onDisconnected: () => { },
		onShutdown: () => {
			runnerStopped.resolve(runner);
		},
		fetch: async (
			runner: Runner,
			actorId: string,
			_gatewayId: ArrayBuffer,
			_requestId: ArrayBuffer,
			request: Request,
		) => {
			getLogger().info(
				`Fetch called for actor ${actorId}, URL: ${request.url}`,
			);
			const url = new URL(request.url);
			if (url.pathname === "/ping") {
				// Return the actor ID in response
				const responseData = {
					actorId,
					status: "ok",
					timestamp: Date.now(),
				};

				return new Response(JSON.stringify(responseData), {
					status: 200,
					headers: { "Content-Type": "application/json" },
				});
			} else if (url.pathname === "/sleep") {
				runner.sleepActor(actorId);

				return new Response("ok", {
					status: 200,
					headers: { "Content-Type": "application/json" },
				});
			}

			return new Response("ok", { status: 200 });
		},
		onActorStart: async (
			_actorId: string,
			_generation: number,
			_config: ActorConfig,
		) => {
			getLogger().info(
				`Actor ${_actorId} started (generation ${_generation})`,
			);
		},
		onActorStop: async (_actorId: string, _generation: number) => {
			getLogger().info(
				`Actor ${_actorId} stopped (generation ${_generation})`,
			);
		},
		websocket: async (
			runner: Runner,
			actorId: string,
			ws: WebSocket,
			_gatewayId: ArrayBuffer,
			_requestId: ArrayBuffer,
			_request: Request,
		) => {
			getLogger().info(`WebSocket connected for actor ${actorId}`);

			// Echo server - send back any messages received
			ws.addEventListener("message", (event) => {
				const data = event.data;
				getLogger().info({
					msg: `WebSocket message from actor ${actorId}`,
					data,
					index: (event as any).rivetMessageIndex,
				});

				ws.send(`Echo: ${data}`);

				// Ack
				const websocketId = Buffer.from(
					(event as any).rivetRequestId,
				).toString("base64");
				websocketLastMsgIndexes.set(
					websocketId,
					(event as any).rivetMessageIndex,
				);
				runner.sendHibernatableWebSocketMessageAck(
					(event as any).rivetGatewayId,
					(event as any).rivetRequestId,
					(event as any).rivetMessageIndex,
				);
			});

			ws.addEventListener("close", () => {
				getLogger().info(`WebSocket closed for actor ${actorId}`);
			});

			ws.addEventListener("error", (error) => {
				getLogger().error({
					msg: `WebSocket error for actor ${actorId}:`,
					error,
				});
			});
		},
		hibernatableWebSocket: {
			canHibernate() {
				return true;
			},
		},
	};

	runner = new Runner(config);

	// Start runner
	await runner.start();

	// Wait for runner to be ready
	getLogger().info("Waiting runner start...");
	await runnerStarted.promise;

	getLogger().info("Runner started");

	return runner;
}

export default app;
