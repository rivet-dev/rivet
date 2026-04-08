import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { EngineActorDriver } from "@/drivers/engine/mod";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { createClientWithDriver } from "@/client/client";
import { updateRunnerConfig } from "@/engine-client/api-endpoints";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";
import invariant from "invariant";
import { describe } from "vitest";
import { getDriverRegistryVariants } from "./driver-registry-variants";

for (const registryVariant of getDriverRegistryVariants(__dirname)) {
	const describeVariant = registryVariant.skip
		? describe.skip
		: describe;
	const variantName = registryVariant.skipReason
		? `${registryVariant.name} (${registryVariant.skipReason})`
		: registryVariant.name;

	describeVariant(`registry (${variantName})`, () => {
		runDriverTests({
			useRealTimers: true,
			isDynamic: registryVariant.name === "dynamic",
			features: {
				hibernatableWebSocketProtocol: true,
			},
			// TODO: Re-enable cbor and json once metadata init delay is eliminated
			encodings: ["bare"],
			clientTypes: ["http"],
			async start() {
				return await createTestRuntime(
					registryVariant.registryPath,
					async (registry) => {
						const endpoint =
							process.env.RIVET_ENDPOINT ||
							"http://127.0.0.1:6420";
						const namespace = `test-${crypto.randomUUID().slice(0, 8)}`;
						const poolName =
							process.env.RIVET_POOL_NAME || "test-driver";
						const token =
							process.env.RIVET_TOKEN || "dev";

						// Create a fresh namespace for test isolation
						const nsResp = await fetch(`${endpoint}/namespaces`, {
							method: "POST",
							headers: {
								"Content-Type": "application/json",
								Authorization: `Bearer ${token}`,
							},
							body: JSON.stringify({ name: namespace, display_name: namespace }),
						});
						if (!nsResp.ok) {
							throw new Error(`Create namespace failed: ${nsResp.status} ${await nsResp.text()}`);
						}

						// Configure registry
						registry.config.endpoint = endpoint;
						registry.config.namespace = namespace;
						registry.config.token = token;
						registry.config.envoy = {
							...registry.config.envoy,
							poolName,
						};

						const parsedConfig = registry.parseConfig();
						const clientConfig = convertRegistryConfigToClientConfig(parsedConfig);
						const engineClient = new RemoteEngineControlClient(clientConfig);
						const inlineClient = createClientWithDriver(engineClient, clientConfig);

						// Start the EngineActorDriver
						const actorDriver = new EngineActorDriver(
							parsedConfig,
							engineClient,
							inlineClient,
						);

						// Start serverless HTTP server
						const app = new Hono();
						app.get("/health", (c) => c.text("ok"));
						app.get("/metadata", (c) =>
							c.json({ runtime: "rivetkit", version: "1", envoyProtocolVersion: 1 }),
						);
						app.post("/start", async (c) => {
							return actorDriver.serverlessHandleStart(c);
						});

						const server = honoServe({
							fetch: app.fetch,
							hostname: "127.0.0.1",
							port: 0,
						});
						if (!server.listening) {
							await new Promise<void>((resolve) => {
								server.once("listening", () => resolve());
							});
						}
						const address = server.address();
						invariant(
							address && typeof address !== "string",
							"missing server address",
						);
						const port = address.port;
						const serverlessUrl = `http://127.0.0.1:${port}`;

						// Register serverless runner with the engine
						await updateRunnerConfig(clientConfig, poolName, {
							datacenters: {
								default: {
									serverless: {
										url: serverlessUrl,
										request_lifespan: 300,
										max_concurrent_actors: 10000,
										slots_per_runner: 1,
										min_runners: 0,
										max_runners: 10000,
									},
								},
							},
						});

						// Wait for envoy to connect
						await actorDriver.waitForReady();

						return {
							rivetEngine: {
								endpoint,
								namespace,
								runnerName: poolName,
								token,
							},
							engineClient,
							hardCrashActor: actorDriver.hardCrashActor.bind(actorDriver),
							cleanup: async () => {
								await actorDriver.shutdown(true);
								await new Promise((resolve) =>
									server.close(() => resolve(undefined)),
								);
							},
						};
					},
				);
			},
		});
	});
}
