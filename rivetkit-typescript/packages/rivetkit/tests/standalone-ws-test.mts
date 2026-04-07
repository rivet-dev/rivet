// Test WebSocket through EngineActorDriver (native envoy path)
// Run: npx tsx tests/standalone-ws-test.mts

import { EngineActorDriver } from "../src/drivers/engine/mod";
import { RemoteEngineControlClient } from "../src/engine-client/mod";
import { convertRegistryConfigToClientConfig } from "../src/client/config";
import { createClientWithDriver } from "../src/client/client";
import { updateRunnerConfig } from "../src/engine-client/api-endpoints";
import { setup, actor } from "../src/mod";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";

const endpoint = "http://127.0.0.1:6420";
const namespace = "default";
const poolName = "test-envoy"; // reuse existing pool that already has metadata
const token = "dev";

// Actor with WebSocket echo
const wsActor = actor({
	state: { msgCount: 0 },
	actions: {
		getCount: (c: any) => c.state.msgCount,
	},
	onWebSocket(ctx: any, ws: any) {
		ws.addEventListener("message", (ev: any) => {
			ctx.state.msgCount++;
			ws.send(`Echo: ${ev.data}`);
		});
	},
});

const registry = setup({ use: { wsActor } });
registry.config.endpoint = endpoint;
registry.config.namespace = namespace;
registry.config.token = token;
registry.config.envoy = { ...registry.config.envoy, poolName };
registry.config.test = { enabled: true };

const parsedConfig = registry.parseConfig();
const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
const engineClient = new RemoteEngineControlClient(clientConfig);
const inlineClient = createClientWithDriver(engineClient, clientConfig);

console.log("Creating EngineActorDriver...");
const actorDriver = new EngineActorDriver(parsedConfig, engineClient, inlineClient);

// Serverless HTTP server
const app = new Hono();
app.get("/metadata", (c: any) => c.json({ runtime: "rivetkit", version: "1", envoyProtocolVersion: 1 }));
app.post("/start", async (c: any) => actorDriver.serverlessHandleStart(c));
const server = honoServe({ fetch: app.fetch, hostname: "127.0.0.1", port: 0 });
await new Promise<void>(r => server.listening ? r() : server.once("listening", r));
const port = (server.address() as any).port;

// Update runner config to point at our server
await updateRunnerConfig(clientConfig, poolName, {
	datacenters: { default: { serverless: {
		url: `http://127.0.0.1:${port}`,
		request_lifespan: 300,
		max_concurrent_actors: 10000,
		slots_per_runner: 1,
		min_runners: 0,
		max_runners: 10000,
	}}},
});

await actorDriver.waitForReady();
console.log("Envoy ready");

// No delay needed - "default" namespace already has metadata from test-envoy

// Test 1: Create actor via API
console.log("\n--- Test: Action ---");
const createResp = await fetch(`${endpoint}/actors?namespace=${namespace}`, {
	method: "POST",
	headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
	body: JSON.stringify({ name: "wsActor", key: `ws-${Date.now()}`, runner_name_selector: poolName, crash_policy: "sleep" }),
});
const actorData = await createResp.json();
const actorId = actorData.actor?.actor_id;
console.log("Created:", createResp.status, actorId?.slice(0, 12));

if (!actorId) {
	console.log("✗ FAIL: no actor ID");
	process.exit(1);
}

// Wait for actor to be ready
await new Promise(r => setTimeout(r, 2000));

// Test action first
const actionResp = await fetch(
	`${endpoint}/gateway/wsActor/action/getCount?rvt-namespace=${namespace}&rvt-method=get&rvt-key=ws-${Date.now().toString().slice(-6)}`,
	{
		method: "POST",
		headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
		body: JSON.stringify(null),
		signal: AbortSignal.timeout(10000),
	},
).catch(e => ({ ok: false, status: 0, text: () => Promise.resolve(e.message) } as any));
console.log("Action:", actionResp.status, actionResp.ok ? "✓" : "✗");

// Test 2: WebSocket
console.log("\n--- Test: WebSocket ---");
const wsEndpoint = endpoint.replace("http://", "ws://");
const ws = new WebSocket(`${wsEndpoint}/ws`, [
	"rivet",
	"rivet_target.actor",
	`rivet_actor.${actorId}`,
	`rivet_token.${token}`,
]);

try {
	const result = await new Promise<string>((resolve, reject) => {
		const timeout = setTimeout(() => reject(new Error("timeout")), 10000);
		ws.addEventListener("open", () => {
			console.log("WS connected, sending message...");
			ws.send("hello from native");
		});
		ws.addEventListener("message", (ev) => {
			clearTimeout(timeout);
			ws.close();
			resolve(ev.data as string);
		});
		ws.addEventListener("error", (e) => {
			clearTimeout(timeout);
			reject(new Error(`WS error: ${(e as any)?.message}`));
		});
		ws.addEventListener("close", (e) => {
			clearTimeout(timeout);
			reject(new Error(`WS closed: ${(e as any)?.code} ${(e as any)?.reason}`));
		});
	});
	console.log("WS response:", result);
	console.log(result.includes("Echo:") ? "✓ PASS" : "✗ FAIL");
} catch (e) {
	console.log("✗ FAIL:", (e as Error).message);
}

await actorDriver.shutdown(true);
server.close();
process.exit(0);
