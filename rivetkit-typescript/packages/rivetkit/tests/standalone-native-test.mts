// Standalone test for native envoy: actions, WebSocket, SQLite
// Uses EngineActorDriver with "default" namespace (metadata already refreshed)
// Run: npx tsx tests/standalone-native-test.mts
//
// Prerequisites:
//   - Engine on localhost:6420 (with force-v2 hack)
//   - Runner config for test-envoy on default namespace with metadata refreshed
//     (run: curl -s -X POST -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
//       http://localhost:6420/runner-configs/test-envoy/refresh-metadata?namespace=default -d '{}')

import { EngineActorDriver } from "../src/drivers/engine/mod";
import { RemoteEngineControlClient } from "../src/engine-client/mod";
import { convertRegistryConfigToClientConfig } from "../src/client/config";
import { createClientWithDriver } from "../src/client/client";
import { createClient } from "../src/client/mod";
import { updateRunnerConfig } from "../src/engine-client/api-endpoints";
import { setup, actor, event } from "../src/mod";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";

const endpoint = "http://127.0.0.1:6420";
const namespace = "default";
const poolName = "test-envoy";
const token = "dev";

// ---- Actors ----
const counter = actor({
	state: { count: 0 },
	events: { newCount: event<number>() },
	actions: {
		increment: (c: any, x: number) => {
			c.state.count += x;
			return c.state.count;
		},
		getCount: (c: any) => c.state.count,
	},
});

// ---- Setup EngineActorDriver ----
const registry = setup({ use: { counter } });
registry.config.endpoint = endpoint;
registry.config.namespace = namespace;
registry.config.token = token;
registry.config.envoy = { ...registry.config.envoy, poolName };
registry.config.test = { enabled: true };

const parsedConfig = registry.parseConfig();
const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
const engineClient = new RemoteEngineControlClient(clientConfig);
const inlineClient = createClientWithDriver(engineClient, clientConfig);

console.log("Starting EngineActorDriver...");
const actorDriver = new EngineActorDriver(parsedConfig, engineClient, inlineClient);

// Serverless HTTP server for the engine to POST start commands
const app = new Hono();
app.get("/metadata", (c: any) => c.json({ runtime: "rivetkit", version: "1", envoyProtocolVersion: 1 }));
app.post("/start", async (c: any) => actorDriver.serverlessHandleStart(c));
const server = honoServe({ fetch: app.fetch, hostname: "127.0.0.1", port: 0 });
await new Promise<void>(r => server.listening ? r() : server.once("listening", r));
const port = (server.address() as any).port;

// Point runner config at our serverless server
await updateRunnerConfig(clientConfig, poolName, {
	datacenters: {
		default: {
			serverless: {
				url: `http://127.0.0.1:${port}`,
				request_lifespan: 300, max_concurrent_actors: 10000,
				slots_per_runner: 1, min_runners: 0, max_runners: 10000,
			}
		}
	},
});

await actorDriver.waitForReady();
console.log(`Ready (serverless on :${port})`);

// Client SDK
const client = createClient<typeof registry>({
	endpoint, namespace, poolName,
	encoding: "json",
	disableMetadataLookup: true,
});

let passed = 0;
let failed = 0;
function ok(name: string) { console.log(`  ✓ ${name}`); passed++; }
function fail(name: string, err: string) { console.log(`  ✗ ${name}: ${err}`); failed++; }

// ---- Test: Action ----
console.log("\n=== Action Tests ===");
try {
	const key = `action-${Date.now()}`;
	const handle = client.counter.getOrCreate([key]);

	const result = await handle.increment(5);
	if (result === 5) ok("increment returns 5");
	else fail("increment returns 5", `got ${result}`);

	const result2 = await handle.increment(3);
	if (result2 === 8) ok("increment accumulates to 8");
	else fail("increment accumulates to 8", `got ${result2}`);

	const count = await handle.getCount();
	if (count === 8) ok("getCount returns 8");
	else fail("getCount returns 8", `got ${count}`);
} catch (e) {
	fail("action test", (e as Error).message?.slice(0, 120));
}

// ---- Test: WebSocket ----
console.log("\n=== WebSocket Tests ===");
try {
	const key = `ws-${Date.now()}`;
	const handle = client.counter.getOrCreate([key]);

	// Create actor first
	await handle.increment(0);

	// Connect
	const conn = handle.connect();

	// Action through existing connection
	const val = await handle.increment(42);
	if (val === 42) ok("action after connect");
	else fail("action after connect", `got ${val}`);

	conn.close();
} catch (e) {
	fail("websocket test", (e as Error).message?.slice(0, 120));
}

// ---- Results ----
console.log(`\n${passed} passed, ${failed} failed`);
await client.dispose();
await actorDriver.shutdown(true);
server.close();
process.exit(failed > 0 ? 1 : 0);
