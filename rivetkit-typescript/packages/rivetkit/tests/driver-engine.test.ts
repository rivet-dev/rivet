import { join } from "node:path";
import { createClientWithDriver } from "@/client/client";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createEngineDriver } from "@/drivers/engine/mod";
import { RunnerConfigSchema } from "@/registry/run-config";

runDriverTests({
	// Use real timers for engine-runner tests
	useRealTimers: true,
	skip: {
		// SSE is not implemented on Rivet Guard yet
		sse: true,
		// The inline client is the same as the remote client driver on Rivet
		inline: true,
	},
	async start() {
		return await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async (registry) => {
				// Get configuration from environment or use defaults
				const endpoint =
					process.env.RIVET_ENDPOINT || "http://127.0.0.1:6420";
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

				// Start the actor driver
				const runConfig = RunnerConfigSchema.parse({
					driver: driverConfig,
					endpoint,
					namespace,
					runnerName,
					token,
					getUpgradeWebSocket: () => undefined,
				});
				const managerDriver = driverConfig.manager(
					registry.config,
					runConfig,
				);
				const inlineClient = createClientWithDriver(
					managerDriver,
					runConfig,
				);
				const actorDriver = driverConfig.actor(
					registry.config,
					runConfig,
					managerDriver,
					inlineClient,
				);

				await new Promise((resolve) => setTimeout(resolve, 1000));

				return {
					rivetEngine: {
						endpoint: "http://127.0.0.1:6420",
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
