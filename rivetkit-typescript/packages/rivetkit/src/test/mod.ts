import { createServer } from "node:net";
import { serve as honoServe } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import invariant from "invariant";
import { type TestContext, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { type Client, createClient } from "@/client/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { buildManagerRouter } from "@/manager/router";
import { createClientWithDriver, type Registry } from "@/mod";
import { RegistryConfig, RegistryConfigSchema } from "@/registry/config";
import { logger } from "./log";

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
}

// Must use `TestContext` since global hooks do not work when running concurrently
export async function setupTest<A extends Registry<any>>(
	c: TestContext,
	registry: A,
): Promise<SetupTestResult<A>> {
	// Force enable test mode
	registry.config.test.enabled = true;

	// Create driver
	const driver = await createFileSystemOrMemoryDriver(
		true,
		`/tmp/rivetkit-test-${crypto.randomUUID()}`,
	);

	// Build driver config
	// biome-ignore lint/style/useConst: Assigned later
	let upgradeWebSocket: any;
	registry.config.driver = driver;
	registry.config.inspector = {
		enabled: true,
		token: () => "token",
	};

	// Create router
	const managerDriver = driver.manager?.(registry.config);
	invariant(managerDriver, "missing manager driver");
	// const internalClient = createClientWithDriver(
	// 	managerDriver,
	// 	ClientConfigSchema.parse({}),
	// );
	const { router } = buildManagerRouter(
		registry.config,
		managerDriver,
		() => upgradeWebSocket!,
	);

	// Inject WebSocket
	const nodeWebSocket = createNodeWebSocket({ app: router });
	upgradeWebSocket = nodeWebSocket.upgradeWebSocket;

	// Start server
	const port = await getPort();
	const server = honoServe({
		fetch: router.fetch,
		hostname: "127.0.0.1",
		port,
	});
	invariant(
		nodeWebSocket.injectWebSocket !== undefined,
		"should have injectWebSocket",
	);
	nodeWebSocket.injectWebSocket(server);
	const endpoint = `http://127.0.0.1:${port}`;

	logger().info({ msg: "test server listening", port });

	// Cleanup on test finish
	c.onTestFinished(async () => {
		await new Promise((resolve) => server.close(() => resolve(undefined)));
	});

	// Create client
	const client = createClient<A>({
		endpoint,
		namespace: "default",
		runnerName: "default",
	});
	c.onTestFinished(async () => await client.dispose());

	return { client };
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
