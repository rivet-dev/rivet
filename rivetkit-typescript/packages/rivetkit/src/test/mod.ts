import { createServer } from "node:net";
import { serve as honoServe, type ServerType } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import { type TestContext, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { type Client, createClient } from "@/client/mod";
import { chooseDefaultDriver } from "@/drivers/default";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import {
	configureInspectorAccessToken,
	getInspectorUrl,
} from "@/inspector/utils";
import { createManagerRouter } from "@/manager/router";
import { createClientWithDriver } from "@/mod";
import type { Registry } from "@/registry/mod";
import { RunnerConfigSchema } from "@/registry/run-config";
import { ConfigSchema, type InputConfig } from "./config";
import { logger } from "./log";

async function serve(
	registry: Registry<any>,
	inputConfig?: InputConfig,
): Promise<ServerType> {
	// Configure default configuration
	inputConfig ??= {};

	const config = ConfigSchema.parse(inputConfig);

	let upgradeWebSocket: any;
	if (!config.getUpgradeWebSocket) {
		config.getUpgradeWebSocket = () => upgradeWebSocket!;
	}

	// Create router
	const runConfig = RunnerConfigSchema.parse(inputConfig);
	const driver =
		inputConfig.driver ?? (await createFileSystemOrMemoryDriver(false));
	const managerDriver = driver.manager(registry.config, config);
	const client = createClientWithDriver(
		managerDriver,
		ClientConfigSchema.parse({}),
	);
	configureInspectorAccessToken(config, managerDriver);
	const { router } = createManagerRouter(
		registry.config,
		runConfig,
		managerDriver,
		driver,
		client,
	);

	// Inject WebSocket
	const nodeWebSocket = createNodeWebSocket({ app: router });
	upgradeWebSocket = nodeWebSocket.upgradeWebSocket;

	const server = honoServe({
		fetch: router.fetch,
		hostname: config.hostname,
		port: config.port,
	});
	nodeWebSocket.injectWebSocket(server);

	logger().info({
		msg: "rivetkit started",
		hostname: config.hostname,
		port: config.port,
		definitions: Object.keys(registry.config.use).length,
	});

	return server;
}

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
	mockDriver: {
		actorDriver: {
			setCreateVarsContext: (ctx: any) => void;
		};
	};
}

// Must use `TestContext` since global hooks do not work when running concurrently
export async function setupTest<A extends Registry<any>>(
	c: TestContext,
	registry: A,
): Promise<SetupTestResult<A>> {
	vi.useFakeTimers();

	// Set up mock driver for testing createVars context
	const mockDriverContext: any = {};
	const setDriverContextFn = (ctx: any) => {
		mockDriverContext.current = ctx;
	};

	// We don't need to modify the driver context anymore since we're testing with the actual context

	// Start server with a random port
	const port = await getPort();
	const server = await serve(registry, { port });
	c.onTestFinished(
		async () =>
			await new Promise((resolve) => server.close(() => resolve())),
	);

	throw "TODO: Fix engine port";

	// // TODO: Figure out how to make this the correct endpoint
	// // Create client
	// const client = createClient<A>(`http://127.0.0.1:${port}`);
	// c.onTestFinished(async () => await client.dispose());
	//
	// return {
	// 	client,
	// 	mockDriver: {
	// 		actorDriver: {
	// 			setCreateVarsContext: setDriverContextFn,
	// 		},
	// 	},
	// };
}

export async function getPort(): Promise<number> {
	// Pick random port between 10000 and 65535 (avoiding well-known and registered ports)
	const MIN_PORT = 10000;
	const MAX_PORT = 65535;
	const getRandomPort = () =>
		Math.floor(Math.random() * (MAX_PORT - MIN_PORT + 1)) + MIN_PORT;

	let port = getRandomPort();
	let maxAttempts = 10;

	while (maxAttempts > 0) {
		try {
			// Try to create a server on the port to check if it's available
			const server = await new Promise<any>((resolve, reject) => {
				const server = createServer();

				server.once("error", (err: Error & { code?: string }) => {
					if (err.code === "EADDRINUSE") {
						reject(new Error(`Port ${port} is in use`));
					} else {
						reject(err);
					}
				});

				server.once("listening", () => {
					resolve(server);
				});

				server.listen(port);
			});

			// Close the server since we're just checking availability
			await new Promise<void>((resolve) => {
				server.close(() => resolve());
			});

			return port;
		} catch (err) {
			// If port is in use, try a different one
			maxAttempts--;
			if (maxAttempts <= 0) {
				break;
			}
			port = getRandomPort();
		}
	}

	throw new Error("Could not find an available port after multiple attempts");
}
