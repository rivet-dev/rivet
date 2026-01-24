import { join } from "node:path";
import { createClientWithDriver } from "@/client/client";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createEngineDriver } from "@/drivers/engine/mod";
import { LegacyRunnerConfigSchema } from "@/registry/config/legacy-runner";
import invariant from "invariant";
import { RegistryConfigSchema } from "@/registry/config";
import {
	ClientConfigSchema,
	convertRegistryConfigToClientConfig,
} from "@/client/config";

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
				// Get configuration from environment or use defaults
				const endpoint =
					process.env.RIVET_ENDPOINT || "http://localhost:6420";
				const namespace = `test-${crypto.randomUUID().slice(0, 8)}`;
				const runnerName = "test-runner";
				const token = "dev";

				// Create namespace
				const response = await fetch(`${endpoint}/namespaces`, {
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
					throw "Create namespace failed";
				}

				// Create driver config
				const driverConfig = createEngineDriver();

				// TODO: We should not have to do this, we should have access to the Runtime instead
				const parsedConfig = registry.parseConfig();

				// Start the actor driver
				registry.config.driver = driverConfig;
				registry.config.endpoint = endpoint;
				registry.config.namespace = namespace;
				registry.config.token = token;
				registry.config.runner = {
					...registry.config.runner,
					runnerName,
				};
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

				await new Promise((resolve) => setTimeout(resolve, 1000));

				return {
					rivetEngine: {
						endpoint: "http://localhost:6420",
						namespace: namespace,
						runnerName: runnerName,
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
