import { serve as honoServe } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import invariant from "invariant";
import { type TestContext, vi } from "vitest";
import { ClientConfigSchema } from "@/client/config";
import { type Client, createClient } from "@/client/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { createClientWithDriver, type Registry } from "@/mod";
import { RegistryConfig, RegistryConfigSchema } from "@/registry/config";
import { buildManagerRouter } from "@/manager/router";
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
	registry.config.test = { ...registry.config.test, enabled: true };

	// Create driver
	const driver = createFileSystemOrMemoryDriver(
		true,
		{ path: `/tmp/rivetkit-test-${crypto.randomUUID()}` },
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
	const parsedConfig = registry.parseConfig();
	const managerDriver = driver.manager?.(parsedConfig);
	invariant(managerDriver, "missing manager driver");
	const getUpgradeWebSocket = () => upgradeWebSocket;
	managerDriver.setGetUpgradeWebSocket(getUpgradeWebSocket);
	// const internalClient = createClientWithDriver(
	// 	managerDriver,
	// 	ClientConfigSchema.parse({}),
	// );
	const { router } = buildManagerRouter(
		parsedConfig,
		managerDriver,
		getUpgradeWebSocket,
	);

	// Inject WebSocket
	const nodeWebSocket = createNodeWebSocket({ app: router });
	upgradeWebSocket = nodeWebSocket.upgradeWebSocket;

	// TODO: I think this whole function is fucked, we should probably switch to calling registry.serve() directly
	// Start server
	const server = honoServe({
		fetch: router.fetch,
		hostname: "127.0.0.1",
		port: 0,
	});
	if (!server.listening) {
		await new Promise<void>((resolve) => {
			server.once("listening", () => resolve());
		});
	}
	invariant(
		nodeWebSocket.injectWebSocket !== undefined,
		"should have injectWebSocket",
	);
	nodeWebSocket.injectWebSocket(server);
	const address = server.address();
	invariant(address && typeof address !== "string", "missing server address");
	const port = address.port;
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
		disableMetadataLookup: true,
	});
	c.onTestFinished(async () => await client.dispose());

	return { client };
}
