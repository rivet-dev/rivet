/**
 * Smoke test that provisions its own serverless runner config, then verifies
 * the native envoy client can route raw HTTP and raw WebSocket traffic through
 * the current gateway URL flow.
 */
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";
import invariant from "invariant";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { WS_PROTOCOL_ENCODING, WS_PROTOCOL_STANDARD } from "@/driver-helpers/mod";
import { EngineActorDriver } from "@/drivers/engine/mod";
import { updateRunnerConfig } from "@/engine-client/api-endpoints";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { createClientWithDriver } from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { createClient } from "@/client/mod";
import { actor, setup } from "@/mod";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { importWebSocket } from "@/common/websocket";

const RIVET_ENDPOINT = process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420";
const RIVET_TOKEN = process.env.RIVET_TOKEN ?? "dev";

const thingy = actor({
	onRequest(_c, request) {
		const pathname = new URL(request.url).pathname;
		if (pathname.endsWith("/ping")) {
			return Response.json({ status: "ok" });
		}

		return new Response("Not Found", { status: 404 });
	},
	onWebSocket(_c, websocket) {
		websocket.addEventListener("message", (event) => {
			websocket.send(`Echo: ${String(event.data)}`);
		});
	},
});

const registry = setup({
	use: {
		thingy,
	},
});

function buildGatewayRequestUrl(gatewayUrl: string, path: string): string {
	const url = new URL(gatewayUrl);
	const normalizedPath = path.replace(/^\//, "");
	url.pathname = `${url.pathname.replace(/\/$/, "")}/request/${normalizedPath}`;
	return url.toString();
}

function buildGatewayWebSocketUrl(gatewayUrl: string, path = ""): string {
	const url = new URL(gatewayUrl);
	url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
	const normalizedPath = path.replace(/^\//, "");
	url.pathname = `${url.pathname.replace(/\/$/, "")}/websocket/${normalizedPath}`;
	return url.toString();
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) {
		return;
	}

	await new Promise<void>((resolve, reject) => {
		const onOpen = () => {
			cleanup();
			resolve();
		};
		const onError = () => {
			cleanup();
			reject(new Error("websocket error before open"));
		};
		const onClose = (event: Event) => {
			const closeEvent = event as CloseEvent;
			cleanup();
			reject(
				new Error(
					`websocket closed before open (${closeEvent.code} ${closeEvent.reason})`,
				),
			);
		};
		const cleanup = () => {
			ws.removeEventListener("open", onOpen);
			ws.removeEventListener("error", onError);
			ws.removeEventListener("close", onClose);
		};

		ws.addEventListener("open", onOpen, { once: true });
		ws.addEventListener("error", onError, { once: true });
		ws.addEventListener("close", onClose, { once: true });
	});
}

async function closeNodeServer(
	server: ReturnType<typeof honoServe>,
): Promise<void> {
	await new Promise<void>((resolve, reject) => {
		server.close((error) => {
			if (error) {
				reject(error);
				return;
			}
			resolve();
		});

		server.closeIdleConnections?.();
		server.closeAllConnections?.();
	});
}

async function refreshRunnerMetadata(
	endpoint: string,
	namespace: string,
	token: string,
	poolName: string,
): Promise<void> {
	let lastError: unknown;

	for (let attempt = 0; attempt < 20; attempt += 1) {
		try {
			const response = await fetch(
				`${endpoint}/runner-configs/${encodeURIComponent(poolName)}/refresh-metadata?namespace=${encodeURIComponent(namespace)}`,
				{
					method: "POST",
					headers: {
						Authorization: `Bearer ${token}`,
						"Content-Type": "application/json",
					},
					body: JSON.stringify({}),
					signal: AbortSignal.timeout(2_000),
				},
			);
			if (response.ok) {
				return;
			}
			lastError = new Error(
				`refresh runner metadata failed: ${response.status} ${await response.text()}`,
			);
		} catch (error) {
			lastError = error;
		}

		if (attempt < 19) {
			await new Promise((resolve) => setTimeout(resolve, 250));
		}
	}

	throw lastError;
}

type SmokeClient = ReturnType<typeof createClient<typeof registry>>;

let client: SmokeClient | undefined;
let actorDriver: EngineActorDriver | undefined;
let server: ReturnType<typeof honoServe> | undefined;

describe("engine driver smoke test", () => {
	beforeAll(async () => {
		const namespace = `test-smoke-${crypto.randomUUID().slice(0, 8)}`;
		const poolName = `test-smoke-${crypto.randomUUID().slice(0, 8)}`;

		const nsResp = await fetch(`${RIVET_ENDPOINT}/namespaces`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${RIVET_TOKEN}`,
			},
			body: JSON.stringify({
				name: namespace,
				display_name: namespace,
			}),
		});
		if (!nsResp.ok) {
			throw new Error(
				`create namespace failed: ${nsResp.status} ${await nsResp.text()}`,
			);
		}

		registry.config.endpoint = RIVET_ENDPOINT;
		registry.config.namespace = namespace;
		registry.config.token = RIVET_TOKEN;
		registry.config.envoy = {
			...registry.config.envoy,
			poolName,
		};

		const parsedConfig = registry.parseConfig();
		const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
		const engineClient = new RemoteEngineControlClient(clientConfig);
		const inlineClient = createClientWithDriver(engineClient, clientConfig);

		actorDriver = new EngineActorDriver(
			parsedConfig,
			engineClient,
			inlineClient,
		);

		const app = new Hono();
		app.get("/health", (c) => handleHealthRequest(c));
		app.get("/metadata", (c) =>
			handleMetadataRequest(
				c,
				parsedConfig,
				{ serverless: {} },
				parsedConfig.publicEndpoint,
				parsedConfig.publicNamespace,
				parsedConfig.publicToken,
			),
		);
		app.post("/start", async (c) => {
			invariant(actorDriver, "missing actor driver");
			return await actorDriver.serverlessHandleStart!(c);
		});

		server = honoServe({
			fetch: app.fetch,
			hostname: "127.0.0.1",
			port: 0,
		});
		if (!server.listening) {
			await new Promise<void>((resolve) => {
				server!.once("listening", () => resolve());
			});
		}
		const address = server.address();
		invariant(address && typeof address !== "string", "missing server address");
		const serverlessUrl = `http://127.0.0.1:${address.port}`;

		await updateRunnerConfig(clientConfig, poolName, {
			datacenters: {
				default: {
					serverless: {
						url: serverlessUrl,
						headers: {},
						request_lifespan: 300,
						slots_per_runner: 1,
						min_runners: 0,
						max_runners: 10000,
						runners_margin: 0,
					},
				},
			},
		});

		await actorDriver.waitForReady();
		await refreshRunnerMetadata(
			RIVET_ENDPOINT,
			namespace,
			RIVET_TOKEN,
			poolName,
		);

		client = createClient<typeof registry>({
			endpoint: RIVET_ENDPOINT,
			namespace,
			poolName,
			disableMetadataLookup: true,
			encoding: "bare",
		});
	}, 30_000);

	afterAll(async () => {
		await client?.dispose();
		await actorDriver?.shutdown(true);
		if (server) {
			await closeNodeServer(server);
		}
	});

	it(
		"HTTP ping returns JSON response",
		async () => {
			invariant(client, "missing smoke test client");
			const handle = client.thingy.getOrCreate([crypto.randomUUID()]);
			const response = await fetch(
				buildGatewayRequestUrl(await handle.getGatewayUrl(), "ping"),
			);

			expect(response.ok).toBe(true);
			await expect(response.json()).resolves.toEqual({ status: "ok" });
		},
		30_000,
	);

	it(
		"WebSocket echo works",
		async () => {
			invariant(client, "missing smoke test client");
			const WebSocket = await importWebSocket();
			const handle = client.thingy.getOrCreate([crypto.randomUUID()]);
			const ws = new WebSocket(
				buildGatewayWebSocketUrl(await handle.getGatewayUrl()),
				[
					WS_PROTOCOL_STANDARD,
					`${WS_PROTOCOL_ENCODING}bare`,
				],
			) as WebSocket;

			await waitForOpen(ws);

			const result = await new Promise<string>((resolve, reject) => {
				const timeout = setTimeout(() => {
					reject(new Error("websocket timeout"));
				}, 10_000);

				ws.addEventListener(
					"message",
					(event: MessageEvent) => {
						clearTimeout(timeout);
						ws.close();
						resolve(String(event.data));
					},
					{ once: true },
				);
				ws.addEventListener(
					"error",
					() => {
						clearTimeout(timeout);
						reject(new Error("websocket error"));
					},
					{ once: true },
				);

				ws.send("ping");
			});

			expect(result).toBe("Echo: ping");
		},
		30_000,
	);
});
