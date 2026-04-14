import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { EngineActorDriver } from "@/drivers/engine/mod";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { createClientWithDriver } from "@/client/client";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { updateRunnerConfig } from "@/engine-client/api-endpoints";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";
import invariant from "invariant";
import { describe } from "vitest";
import { getDriverRegistryVariants } from "./driver-registry-variants";

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

		// Force-close keep-alive sockets so test cleanup does not hang behind
		// idle serverless connections after actor shutdown.
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

for (const registryVariant of getDriverRegistryVariants(__dirname)) {
	const describeVariant = registryVariant.skip
		? describe.skip
		: describe.sequential;
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
							process.env.RIVET_POOL_NAME ||
							`test-driver-${crypto.randomUUID().slice(0, 8)}`;
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
						let actorDriver: EngineActorDriver | undefined;

						// Start serverless HTTP server
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
							return actorDriver.serverlessHandleStart(c);
						});
						app.post("/.test/native-db/force-disconnect", async (c) => {
							invariant(actorDriver, "missing actor driver");
							const closed =
								await actorDriver.forceDisconnectNativeDatabaseTransportForTests?.();
							return c.json({ closed: closed ?? 0 });
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

						// Start the EngineActorDriver after the serverless pool exists so the
						// envoy connection is classified as serverless on first connect.
						actorDriver = new EngineActorDriver(
							parsedConfig,
							engineClient,
							inlineClient,
						);

						// Wait for envoy to connect
						await actorDriver.waitForReady();

						try {
							await refreshRunnerMetadata(
								endpoint,
								namespace,
								token,
								poolName,
							);
						} catch {
							// The engine can take a while to expose the metadata refresh
							// endpoint in local test harnesses. The per-test warmup actor
							// probe is the real readiness barrier.
						}

						return {
							rivetEngine: {
								endpoint,
								testEndpoint: serverlessUrl,
								namespace,
								runnerName: poolName,
								token,
							},
							engineClient,
							hardCrashActor: actorDriver.hardCrashActor.bind(actorDriver),
							cleanup: async () => {
								await actorDriver.shutdown(true);
								await closeNodeServer(server);
							},
						};
					},
				);
			},
		});
	});
}
