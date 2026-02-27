import { join } from "node:path";
import { createClientWithDriver } from "@/client/client";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createEngineDriver } from "@/drivers/engine/mod";
import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";

runDriverTests({
	// Use real timers for engine-runner tests
	useRealTimers: true,
	skip: {
		// The inline client is the same as the remote client driver on Rivet
		inline: true,
	},
	async start() {
		return await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async (registry) => {
				// Get configuration from environment or use defaults.
				const endpoint = process.env.RIVET_ENDPOINT || "http://127.0.0.1:6420";
				const namespaceEndpoint =
					process.env.RIVET_NAMESPACE_ENDPOINT ||
					process.env.RIVET_API_ENDPOINT ||
					endpoint;
				const namespace = `test-${crypto.randomUUID().slice(0, 8)}`;
				const runnerName = "test-runner";
				const token = "dev";

				// Create namespace.
				const response = await fetch(`${namespaceEndpoint}/namespaces`, {
					method: "POST",
					headers: {
						"Content-Type": "application/json",
						Authorization: "Bearer dev",
					},
					body: JSON.stringify({
						name: namespace,
						display_name: namespace,
					}),
				});
				if (!response.ok) {
					const errorBody = await response.text().catch(() => "");
					throw new Error(
						`Create namespace failed at ${namespaceEndpoint}: ${response.status} ${response.statusText} ${errorBody}`,
					);
				}

				// Create driver config.
				const driverConfig = createEngineDriver();

				// Start the actor driver.
				registry.config.driver = driverConfig;
				registry.config.endpoint = endpoint;
				registry.config.namespace = namespace;
				registry.config.token = token;
				registry.config.runner = {
					...registry.config.runner,
					runnerName,
				};

				// Parse config only after mutating registry.config so the manager
				// and actor drivers do not get stale namespace/runner values from
				// previous tests.
				const parsedConfig = registry.parseConfig();

				const managerDriver = driverConfig.manager?.(parsedConfig);
				invariant(managerDriver, "missing manager driver");
				const inlineClient = createClientWithDriver(
					managerDriver,
					convertRegistryConfigToClientConfig(parsedConfig),
				);

				const actorDriver = driverConfig.actor(
					parsedConfig,
					managerDriver,
					inlineClient,
				);

				// Wait for runner registration so tests do not race actor creation
				// against asynchronous runner connect.
				const runnersUrl = new URL(`${endpoint.replace(/\/$/, "")}/runners`);
				runnersUrl.searchParams.set("namespace", namespace);
				runnersUrl.searchParams.set("name", runnerName);
				let probeError: unknown;
				for (let attempt = 0; attempt < 120; attempt++) {
					try {
						const runnerResponse = await fetch(runnersUrl, {
							method: "GET",
							headers: {
								Authorization: `Bearer ${token}`,
							},
						});
						if (!runnerResponse.ok) {
							const errorBody = await runnerResponse.text().catch(() => "");
							probeError = new Error(
								`List runners failed: ${runnerResponse.status} ${runnerResponse.statusText} ${errorBody}`,
							);
						} else {
							const responseJson = (await runnerResponse.json()) as {
								runners?: Array<{ name?: string }>;
							};
							const hasRunner = !!responseJson.runners?.some(
								(runner) => runner.name === runnerName,
							);
							if (hasRunner) {
								probeError = undefined;
								break;
							}
							probeError = new Error(
								`Runner ${runnerName} not registered yet`,
							);
						}
					} catch (err) {
						probeError = err;
					}
					if (attempt < 119) {
						await new Promise((resolve) => setTimeout(resolve, 100));
					}
				}
				if (probeError) {
					throw probeError;
				}

				return {
					rivetEngine: {
						endpoint,
						namespace,
						runnerName,
						token,
					},
					driver: driverConfig,
					cleanup: async () => {
						await actorDriver.shutdownRunner?.(true);
					},
				};
			},
		);
	},
});
