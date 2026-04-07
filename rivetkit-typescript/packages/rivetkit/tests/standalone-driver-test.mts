// Standalone test - run with: npx tsx tests/standalone-driver-test.mts
// Tests EngineActorDriver OUTSIDE vitest to isolate the issue

import { EngineActorDriver } from "../src/drivers/engine/mod";
import { RemoteEngineControlClient } from "../src/engine-client/mod";
import { convertRegistryConfigToClientConfig } from "../src/client/config";
import { createClientWithDriver } from "../src/client/client";
import { updateRunnerConfig } from "../src/engine-client/api-endpoints";
import { setup, actor } from "../src/mod";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";

const endpoint = "http://127.0.0.1:6420";
const namespace = process.env.TEST_NS || `test-${crypto.randomUUID().slice(0, 8)}`;
const poolName = "test-driver";
const token = "dev";

// Create namespace if needed
if (!process.env.TEST_NS) {
  const nsResp = await fetch(`${endpoint}/namespaces`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({ name: namespace, display_name: namespace }),
  });
  console.log("Namespace created:", nsResp.status, namespace);
} else {
  console.log("Using existing namespace:", namespace);
}

// Minimal registry with counter actor
const counterActor = actor({
  state: { count: 0 },
  actions: {
    increment: (c: any, x: number) => {
      c.state.count += x;
      return c.state.count;
    },
  },
});

const registry = setup({ use: { counter: counterActor } });
registry.config.endpoint = endpoint;
registry.config.namespace = namespace;
registry.config.token = token;
registry.config.envoy = { ...registry.config.envoy, poolName };
registry.config.test = { enabled: true };

const parsedConfig = registry.parseConfig();
const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
const engineClient = new RemoteEngineControlClient(clientConfig);
const inlineClient = createClientWithDriver(engineClient, clientConfig);

// Create EngineActorDriver
console.log("Creating EngineActorDriver...");
const actorDriver = new EngineActorDriver(parsedConfig, engineClient, inlineClient);

// Start serverless HTTP server
const app = new Hono();
app.get("/health", (c: any) => c.text("ok"));
app.get("/metadata", (c: any) => c.json({ runtime: "rivetkit", version: "1", envoyProtocolVersion: 1 }));
app.post("/start", async (c: any) => actorDriver.serverlessHandleStart(c));

const server = honoServe({ fetch: app.fetch, hostname: "127.0.0.1", port: 0 });
await new Promise<void>((resolve) => {
  if (server.listening) resolve();
  else server.once("listening", resolve);
});
const address = server.address() as any;
const port = address.port;
console.log("Serverless server on port:", port);

// Register runner config
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
console.log("Runner config updated");

// Wait for envoy
await actorDriver.waitForReady();
console.log("Envoy ready");

// Refresh metadata so engine knows our protocol version (enables v2 POST path)
const refreshResp = await fetch(
  `${endpoint}/runner-configs/${poolName}/refresh-metadata?namespace=${namespace}`,
  {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({}),
  },
);
console.log("Metadata refreshed:", refreshResp.status);

// Wait for engine to process the metadata and start the runner pool
await new Promise(r => setTimeout(r, 5000));

// Create actor via gateway (exactly what the client does)
console.log("Creating actor via gateway...");
const start = Date.now();
const gwResp = await fetch(
  `${endpoint}/gateway/counter/action/increment?rvt-namespace=${namespace}&rvt-method=getOrCreate&rvt-runner=${poolName}&rvt-crash-policy=sleep`,
  {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify(5),
    signal: AbortSignal.timeout(15000),
  },
);
const elapsed = Date.now() - start;
console.log(`Gateway response: HTTP ${gwResp.status} in ${elapsed}ms`);
console.log("Body:", (await gwResp.text()).slice(0, 100));

// Cleanup
await actorDriver.shutdown(true);
server.close();
process.exit(gwResp.ok ? 0 : 1);
