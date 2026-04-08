import * as protocol from "@rivetkit/engine-envoy-protocol";
import { ShutdownReason, EnvoyHandle, startEnvoy, startEnvoySync } from "@rivetkit/engine-envoy-client";
import { Hono, type Context as HonoContext, type Next } from "hono";
import { streamSSE } from "hono/streaming";
import type { Logger } from "pino";
import type WebSocket from "ws";
import { getLogger } from "./log";

const INTERNAL_SERVER_PORT = process.env.INTERNAL_SERVER_PORT
	? Number(process.env.INTERNAL_SERVER_PORT)
	: 5051;
const RIVET_NAMESPACE = process.env.RIVET_NAMESPACE ?? "default";
const RIVET_POOL_NAME = process.env.RIVET_POOL_NAME ?? "test-envoy";
const RIVET_ENVOY_VERSION = process.env.RIVET_ENVOY_VERSION
	? Number(process.env.RIVET_ENVOY_VERSION)
	: 1;
const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";
const AUTOSTART_SERVER = (process.env.AUTOSTART_SERVER ?? "1") == "1";
const AUTOSTART_ENVOY = (process.env.AUTOSTART_ENVOY ?? "0") == "1";
const AUTOCONFIGURE_SERVERLESS = (process.env.AUTOCONFIGURE_SERVERLESS ?? "1") == "1";

let envoy: EnvoyHandle | null = null;
const websocketLastMsgIndexes: Map<string, number> = new Map();
const config = {
	logger: getLogger(),
	version: RIVET_ENVOY_VERSION,
	endpoint: RIVET_ENDPOINT,
	token: RIVET_TOKEN,
	namespace: RIVET_NAMESPACE,
	poolName: RIVET_POOL_NAME,
	prepopulateActorNames: {},
	fetch: async (
		envoy: EnvoyHandle,
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
			envoy.sleepActor(actorId);

			return new Response("ok", {
				status: 200,
				headers: { "Content-Type": "application/json" },
			});
		}

		return new Response("ok", { status: 200 });
	},
	onActorStart: async (
		_envoy: EnvoyHandle,
		_actorId: string,
		_generation: number,
		_config: protocol.ActorConfig,
	) => {
		getLogger().info(
			`Actor ${_actorId} started (generation ${_generation})`,
		);
	},
	onActorStop: async (
		_envoy: EnvoyHandle,
		_actorId: string,
		_generation: number,
		reason: protocol.StopActorReason,
	) => {
		getLogger().info(
			`Actor ${_actorId} stopped (generation ${_generation})`,
		);
	},
	onShutdown() { },
	websocket: async (
		envoy: EnvoyHandle,
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
			envoy.sendHibernatableWebSocketMessageAck(
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

app.get("/health", (c) => {
	return c.text("ok");
});

app.get("/shutdown", async (c) => {
	envoy?.shutdown(false);
	return c.text("ok");
});

app.post("/api/rivet/start", async (c) => {
	getLogger().info({
		msg: `Received SSE request`,
	});

	let payload = await c.req.arrayBuffer();

	return streamSSE(c, async (stream) => {
		c.req.raw.signal.addEventListener("abort", () => {
			getLogger().debug("SSE aborted");
		});

		const envoy = startEnvoySync({
			...config,
		});

		envoy.startServerlessActor(payload);

		while (true) {
			if (stream.closed || stream.aborted) break;

			await stream.writeSSE({ event: "ping", data: "" });
			await stream.sleep(1000);
		}
	});
});

app.get("/api/rivet/metadata", async (c) => {
	return c.json({
		// Not actually rivetkit
		runtime: "rivetkit",
		version: "1",
		envoyProtocolVersion: protocol.VERSION,
	});
});

if (AUTOSTART_SERVER) {
	if (process.versions.bun) {
		Bun.serve({ fetch: app.fetch, port: INTERNAL_SERVER_PORT, idleTimeout: 0 });
	} else {
		const { serve } = await import("@hono/node-server");
		serve({ fetch: app.fetch, port: INTERNAL_SERVER_PORT });
	}
	getLogger().info(
		`Internal HTTP server listening on port ${INTERNAL_SERVER_PORT}`,
	);
}

if (AUTOSTART_ENVOY) {
	envoy = await startEnvoy(config);
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
		`${RIVET_ENDPOINT}/runner-configs/${RIVET_POOL_NAME}?namespace=${RIVET_NAMESPACE}`,
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
							url: `http://localhost:${INTERNAL_SERVER_PORT}/api/rivet`,
							request_lifespan: 300,
							max_concurrent_actors: 10000,
							max_runners: 10000,
							slots_per_runner: 1,
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
