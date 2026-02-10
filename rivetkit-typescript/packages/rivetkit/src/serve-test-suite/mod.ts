import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import { serve as honoServe } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import invariant from "invariant";
import { logger } from "@/driver-test-suite/log";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { buildManagerRouter } from "@/manager/router";
import { registry } from "../../fixtures/driver-test-suite/registry";

export interface ServeTestSuiteResult {
	endpoint: string;
	namespace: string;
	runnerName: string;
	close(): Promise<void>;
}

async function getPort(): Promise<number> {
	const MIN_PORT = 10000;
	const MAX_PORT = 65535;
	const getRandomPort = () =>
		Math.floor(Math.random() * (MAX_PORT - MIN_PORT + 1)) + MIN_PORT;

	let port = getRandomPort();
	let maxAttempts = 10;

	while (maxAttempts > 0) {
		try {
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

			await new Promise<void>((resolve) => {
				server.close(() => resolve());
			});

			return port;
		} catch {
			maxAttempts--;
			if (maxAttempts <= 0) {
				break;
			}
			port = getRandomPort();
		}
	}

	throw new Error("Could not find an available port after multiple attempts");
}

export async function serveTestSuite(): Promise<ServeTestSuiteResult> {
	registry.config.test = { ...registry.config.test, enabled: true };
	registry.config.inspector = {
		enabled: true,
		token: () => "token",
	};
	const port = await getPort();
	registry.config.managerPort = port;
	registry.config.serverless = {
		...registry.config.serverless,
		publicEndpoint: `http://127.0.0.1:${port}`,
	};

	const driver = await createFileSystemOrMemoryDriver(true, {
		path: `/tmp/rivetkit-test-suite-${crypto.randomUUID()}`,
	});
	registry.config.driver = driver;

	let upgradeWebSocket: any;

	const parsedConfig = registry.parseConfig();
	const managerDriver = driver.manager?.(parsedConfig);
	invariant(managerDriver, "missing manager driver");
	const { router } = buildManagerRouter(
		parsedConfig,
		managerDriver,
		() => upgradeWebSocket,
	);

	const nodeWebSocket = createNodeWebSocket({ app: router });
	upgradeWebSocket = nodeWebSocket.upgradeWebSocket;
	managerDriver.setGetUpgradeWebSocket(() => upgradeWebSocket);

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

	logger().info({ msg: "test suite server listening", port });

	return {
		endpoint,
		namespace: "default",
		runnerName: "default",
		close: async () => {
			await new Promise((resolve) =>
				server.close(() => resolve(undefined)),
			);
		},
	};
}

async function runCli() {
	const result = await serveTestSuite();
	process.stdout.write(
		`${JSON.stringify({
			endpoint: result.endpoint,
			namespace: result.namespace,
			runnerName: result.runnerName,
		})}\n`,
	);

	const shutdown = async () => {
		await result.close();
		process.exit(0);
	};

	process.on("SIGINT", shutdown);
	process.on("SIGTERM", shutdown);
}

const mainPath = process.argv[1];
if (mainPath && mainPath === fileURLToPath(import.meta.url)) {
	runCli().catch((err) => {
		logger().error({ msg: "serve-test-suite failed", error: err });
		process.exit(1);
	});
}
