// Standalone end-to-end test for the native envoy path.
// Verifies action calls, actor connections, raw WebSockets, and SQLite
// persistence through @rivetkit/rivetkit-napi.
//
// Run: npx tsx tests/standalone-native-test.mts

import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";
import { createClientWithDriver } from "../src/client/client";
import { convertRegistryConfigToClientConfig } from "../src/client/config";
import { createClient } from "../src/client/mod";
import { db } from "../src/db/mod";
import { EngineActorDriver } from "../src/drivers/engine/mod";
import { updateRunnerConfig } from "../src/engine-client/api-endpoints";
import { RemoteEngineControlClient } from "../src/engine-client/mod";
import { actor, setup } from "../src/mod";

const endpoint = "http://127.0.0.1:6420";
const namespace = "default";
const poolName = "test-envoy";
const token = "dev";

const nativeActor = actor({
	state: {
		count: 0,
		lastWebSocketMessage: null as string | null,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS message_log (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					source TEXT NOT NULL,
					message TEXT NOT NULL,
					created_at INTEGER NOT NULL
				);
			`);
		},
	}),
	actions: {
		increment: async (c: any, value: number) => {
			c.state.count += value;
			await c.db.execute(
				"INSERT INTO message_log (source, message, created_at) VALUES (?, ?, ?)",
				"action",
				`increment:${c.state.count}`,
				Date.now(),
			);
			return c.state.count;
		},
		record: async (c: any, source: string, message: string) => {
			await c.db.execute(
				"INSERT INTO message_log (source, message, created_at) VALUES (?, ?, ?)",
				source,
				message,
				Date.now(),
			);
		},
		getSummary: async (c: any) => {
			const countRows = await c.db.execute<{ count: number }>(
				"SELECT COUNT(*) AS count FROM message_log",
			);
			const latestRows = await c.db.execute<{
				source: string;
				message: string;
			}>(
				"SELECT source, message FROM message_log ORDER BY id DESC LIMIT 1",
			);

			return {
				entryCount: Number(countRows[0]?.count ?? 0),
				latest: latestRows[0] ?? null,
				stateCount: c.state.count,
				lastWebSocketMessage: c.state.lastWebSocketMessage,
			};
		},
		getMessages: async (c: any) => {
			return await c.db.execute<{
				id: number;
				source: string;
				message: string;
			}>(
				"SELECT id, source, message FROM message_log ORDER BY id ASC",
			);
		},
	},
	onWebSocket(c: any, ws: WebSocket) {
		ws.addEventListener("message", async (event: MessageEvent) => {
			const message = String(event.data);
			c.state.lastWebSocketMessage = message;
			await c.db.execute(
				"INSERT INTO message_log (source, message, created_at) VALUES (?, ?, ?)",
				"websocket",
				message,
				Date.now(),
			);
			ws.send(
				JSON.stringify({
					ok: true,
					echo: message,
					stateCount: c.state.count,
				}),
			);
		});
	},
});

const registry = setup({ use: { nativeActor } });
registry.config.endpoint = endpoint;
registry.config.namespace = namespace;
registry.config.token = token;
registry.config.envoy = { ...registry.config.envoy, poolName };
registry.config.test = { enabled: true };

const parsedConfig = registry.parseConfig();
const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
const engineClient = new RemoteEngineControlClient(clientConfig);
const inlineClient = createClientWithDriver(engineClient, clientConfig);

const app = new Hono();

app.get("/metadata", (c: any) =>
	c.json({ runtime: "rivetkit", version: "1", envoyProtocolVersion: 1 }),
);
app.post("/start", async (c: any) => actorDriver.serverlessHandleStart(c));

const actorDriver = new EngineActorDriver(parsedConfig, engineClient, inlineClient);
const server = honoServe({ fetch: app.fetch, hostname: "127.0.0.1", port: 0 });

const unexpectedFailures: string[] = [];
const onUnhandledRejection = (error: unknown) => {
	unexpectedFailures.push(
		`unhandled rejection: ${error instanceof Error ? error.stack ?? error.message : String(error)}`,
	);
};
const onUncaughtException = (error: Error) => {
	unexpectedFailures.push(
		`uncaught exception: ${error.stack ?? error.message}`,
	);
};

process.on("unhandledRejection", onUnhandledRejection);
process.on("uncaughtException", onUncaughtException);

let passed = 0;
let failed = 0;

function ok(name: string) {
	console.log(`  ✓ ${name}`);
	passed++;
}

function fail(name: string, error: unknown) {
	const message =
		error instanceof Error ? error.stack ?? error.message : String(error);
	console.log(`  ✗ ${name}: ${message}`);
	failed++;
}

async function waitFor(
	check: () => boolean,
	label: string,
	timeoutMs = 10_000,
): Promise<void> {
	const start = Date.now();
	while (!check()) {
		if (Date.now() - start > timeoutMs) {
			throw new Error(`timed out waiting for ${label}`);
		}
		await new Promise((resolve) => setTimeout(resolve, 25));
	}
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) {
		return;
	}

	await new Promise<void>((resolve, reject) => {
		const timeout = setTimeout(() => {
			cleanup();
			reject(new Error("timed out waiting for raw websocket open"));
		}, 10_000);
		const cleanup = () => {
			clearTimeout(timeout);
			ws.removeEventListener("open", onOpen);
			ws.removeEventListener("error", onError);
			ws.removeEventListener("close", onClose);
		};
		const onOpen = () => {
			cleanup();
			resolve();
		};
		const onError = () => {
			cleanup();
			reject(new Error("raw websocket error before open"));
		};
		const onClose = (event: Event) => {
			const closeEvent = event as CloseEvent;
			cleanup();
			reject(
				new Error(
					`raw websocket closed before open (${closeEvent.code} ${closeEvent.reason})`,
				),
			);
		};

		ws.addEventListener("open", onOpen, { once: true });
		ws.addEventListener("error", onError, { once: true });
		ws.addEventListener("close", onClose, { once: true });
	});
}

async function closeWebSocket(ws: WebSocket): Promise<void> {
	if (
		ws.readyState === WebSocket.CLOSING ||
		ws.readyState === WebSocket.CLOSED
	) {
		return;
	}

	await new Promise<void>((resolve) => {
		ws.addEventListener("close", () => resolve(), { once: true });
		ws.close(1000, "done");
	});
}

let client:
	| ReturnType<typeof createClient<typeof registry>>
	| undefined;
let conn: any;
let rawWs: WebSocket | undefined;

try {
	console.log("Starting EngineActorDriver...");
	await new Promise<void>((resolve) =>
		server.listening ? resolve() : server.once("listening", resolve),
	);
	const port = (server.address() as any).port;

	await updateRunnerConfig(clientConfig, poolName, {
		datacenters: {
			default: {
				serverless: {
					url: `http://127.0.0.1:${port}`,
					request_lifespan: 300,
					max_concurrent_actors: 10000,
					slots_per_runner: 1,
					min_runners: 0,
					max_runners: 10000,
				},
			},
		},
	});

	await actorDriver.waitForReady();
	console.log(`Ready (serverless on :${port})`);

	client = createClient<typeof registry>({
		endpoint,
		namespace,
		poolName,
		encoding: "json",
		disableMetadataLookup: true,
	});

	const key = `native-e2e-${Date.now()}`;
	const handle = client.nativeActor.getOrCreate([key]);

	console.log("\n=== Action + SQLite Tests ===");
	try {
		const value = await handle.increment(5);
		if (value === 5) ok("increment persists count");
		else fail("increment persists count", `got ${value}`);

		await handle.record("action", "manual-record");
		const summary = await handle.getSummary();
		if (summary.entryCount === 2) ok("sqlite records action writes");
		else fail("sqlite records action writes", `got ${summary.entryCount}`);

		if (summary.stateCount === 5) ok("state survives sqlite usage");
		else fail("state survives sqlite usage", `got ${summary.stateCount}`);
	} catch (error) {
		fail("action + sqlite flow", error);
	}

	console.log("\n=== Actor Connection Test ===");
	try {
		conn = handle.connect();
		await waitFor(() => conn.isConnected, "actor connection");

		const value = await conn.increment(7);
		if (value === 12) ok("actor connection action works over websocket");
		else fail("actor connection action works over websocket", `got ${value}`);

		await conn.dispose();
		conn = undefined;
		ok("actor connection disposes cleanly");
	} catch (error) {
		fail("actor connection flow", error);
	}

	console.log("\n=== Raw WebSocket + SQLite Tests ===");
	try {
		rawWs = await handle.webSocket();
		await waitForOpen(rawWs);

		const responsePromise = new Promise<{
			ok: boolean;
			echo: string;
			stateCount: number;
		}>((resolve, reject) => {
			const timeout = setTimeout(() => {
				cleanup();
				reject(new Error("timed out waiting for raw websocket message"));
			}, 10_000);
			const cleanup = () => {
				clearTimeout(timeout);
				rawWs?.removeEventListener("message", onMessage);
				rawWs?.removeEventListener("error", onError);
				rawWs?.removeEventListener("close", onClose);
			};
			const onMessage = (event: MessageEvent) => {
				cleanup();
				resolve(JSON.parse(String(event.data)));
			};
			const onError = () => {
				cleanup();
				reject(new Error("raw websocket error"));
			};
			const onClose = (event: Event) => {
				const closeEvent = event as CloseEvent;
				cleanup();
				reject(
					new Error(
						`raw websocket closed early (${closeEvent.code} ${closeEvent.reason})`,
					),
				);
			};

			rawWs?.addEventListener("message", onMessage);
			rawWs?.addEventListener("error", onError, { once: true });
			rawWs?.addEventListener("close", onClose, { once: true });
		});

		rawWs.send("hello-native");
		const response = await responsePromise;

		if (response.ok && response.echo === "hello-native") {
			ok("raw websocket echoes message");
		} else {
			fail("raw websocket echoes message", JSON.stringify(response));
		}

		if (response.stateCount === 12) ok("raw websocket sees latest actor state");
		else fail("raw websocket sees latest actor state", `got ${response.stateCount}`);

		await closeWebSocket(rawWs);
		rawWs = undefined;

		const summary = await handle.getSummary();
		if (summary.entryCount === 4) ok("raw websocket writes to sqlite");
		else fail("raw websocket writes to sqlite", `got ${summary.entryCount}`);

		if (summary.lastWebSocketMessage === "hello-native") {
			ok("websocket message updates actor state");
		} else {
			fail(
				"websocket message updates actor state",
				`got ${summary.lastWebSocketMessage}`,
			);
		}

		if (
			summary.latest?.source === "websocket" &&
			summary.latest?.message === "hello-native"
		) {
			ok("latest sqlite row comes from raw websocket");
		} else {
			fail("latest sqlite row comes from raw websocket", JSON.stringify(summary.latest));
		}

		const messages = await handle.getMessages();
		const sources = messages.map((entry) => entry.source).join(",");
		if (sources === "action,action,action,websocket") ok("sqlite preserves full write history");
		else fail("sqlite preserves full write history", `got ${sources}`);
	} catch (error) {
		fail("raw websocket + sqlite flow", error);
	}
} finally {
	try {
		if (rawWs) {
			await closeWebSocket(rawWs).catch(() => undefined);
		}
		if (conn) {
			await conn.dispose().catch(() => undefined);
		}
		if (client) {
			await client.dispose().catch(() => undefined);
		}
		await actorDriver.shutdown(false).catch(() => undefined);
		await new Promise<void>((resolve) => server.close(() => resolve()));
		await new Promise((resolve) => setTimeout(resolve, 250));
	} finally {
		process.off("unhandledRejection", onUnhandledRejection);
		process.off("uncaughtException", onUncaughtException);
	}
}
if (unexpectedFailures.length > 0) {
	for (const failure of unexpectedFailures) {
		fail("unexpected runtime failure", failure);
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
