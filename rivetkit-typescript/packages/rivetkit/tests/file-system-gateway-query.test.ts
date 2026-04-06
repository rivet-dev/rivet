import { createServer } from "node:net";
import { join } from "node:path";
import { serve as honoServe } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import invariant from "invariant";
import { afterEach, describe, expect, test } from "vitest";
import { createClientWithDriver } from "@/client/client";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { buildManagerRouter } from "@/manager/router";
import { registry } from "../fixtures/driver-test-suite/registry";

describe.sequential("file-system manager gateway query routing", () => {
	const cleanups: Array<() => Promise<void>> = [];

	afterEach(async () => {
		while (cleanups.length > 0) {
			await cleanups.pop()?.();
		}
	});

	test("getOrCreate gateway URLs stay query-backed for the local manager", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const gatewayUrl = await runtime.client.counter
			.getOrCreate(["gateway-query"])
			.getGatewayUrl();

		expect(new URL(gatewayUrl).pathname).toMatch(/\/gateway\/[^/]+;/);
		expect(gatewayUrl).toContain(";namespace=default;");
		expect(gatewayUrl).toContain(";method=getOrCreate;");
		expect(gatewayUrl).toContain(";crashPolicy=sleep");

		const response = await fetch(`${gatewayUrl}/inspector/state`, {
			headers: { Authorization: "Bearer token" },
		});

		expect(response.status).toBe(200);
		await expect(response.json()).resolves.toEqual({
			state: { count: 0 },
			isStateEnabled: true,
		});
	});

	test("get gateway URLs resolve existing actors through matrix paths", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const createHandle = runtime.client.counter.getOrCreate([
			"existing-query",
		]);
		await createHandle.increment(2);

		const getGatewayUrl = await runtime.client.counter
			.get(["existing-query"])
			.getGatewayUrl();

		expect(new URL(getGatewayUrl).pathname).toMatch(/\/gateway\/[^/]+;/);
		expect(getGatewayUrl).toContain(";namespace=default;");
		expect(getGatewayUrl).toContain(";method=get;");

		const response = await fetch(`${getGatewayUrl}/inspector/state`, {
			headers: { Authorization: "Bearer token" },
		});

		expect(response.status).toBe(200);
		await expect(response.json()).resolves.toEqual({
			state: { count: 2 },
			isStateEnabled: true,
		});
	});

	test("invalid matrix syntax is rejected by the local manager route", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const response = await fetch(
			`${runtime.endpoint}/gateway/counter;namespace=default;method=get;extra=value/inspector/state`,
		);

		expect(response.status).toBe(400);
		await expect(response.json()).resolves.toMatchObject({
			group: "request",
			code: "invalid",
		});
	});

	test("create query gateway paths are rejected by the local manager route", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const response = await fetch(
			`${runtime.endpoint}/gateway/counter;namespace=default;method=create/inspector/state`,
		);

		expect(response.status).toBe(400);
		await expect(response.json()).resolves.toMatchObject({
			group: "request",
			code: "invalid",
		});
	});

	test("WebSocket connections work through query-backed gateway paths", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const handle = runtime.client.counter.getOrCreate(["ws-query"]);
		const connection = handle.connect();

		const count = await connection.increment(3);
		expect(count).toBe(3);

		const count2 = await connection.getCount();
		expect(count2).toBe(3);

		await connection.dispose();
	});

	test("namespace mismatches are rejected by the local manager route", async () => {
		const runtime = await startFileSystemGatewayRuntime();
		cleanups.push(runtime.cleanup);

		const response = await fetch(
			`${runtime.endpoint}/gateway/counter;namespace=wrong;method=get;key=room/inspector/state`,
		);

		expect(response.status).toBe(400);
		await expect(response.json()).resolves.toMatchObject({
			group: "request",
			code: "invalid",
		});
	});
});

async function startFileSystemGatewayRuntime() {
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

	const driver = createFileSystemOrMemoryDriver(true, {
		path: join(
			"/tmp",
			`rivetkit-file-system-gateway-${crypto.randomUUID()}`,
		),
	});
	registry.config.driver = driver;

	let upgradeWebSocket: ReturnType<
		typeof createNodeWebSocket
	>["upgradeWebSocket"];

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
	await waitForServer(server);

	invariant(
		nodeWebSocket.injectWebSocket !== undefined,
		"should have injectWebSocket",
	);
	nodeWebSocket.injectWebSocket(server);

	const client = createClientWithDriver<typeof registry>(managerDriver);

	return {
		endpoint: `http://127.0.0.1:${port}`,
		client,
		cleanup: async () => {
			await client.dispose().catch(() => undefined);
			await closeServer(server);
		},
	};
}

async function getPort(): Promise<number> {
	const server = createServer();

	try {
		await new Promise<void>((resolve, reject) => {
			server.once("error", reject);
			server.listen(0, "127.0.0.1", () => resolve());
		});

		const address = server.address();
		if (!address || typeof address === "string") {
			throw new Error("missing test port");
		}

		return address.port;
	} finally {
		await closeServer(server);
	}
}

async function closeServer(server: {
	close(callback: (error?: Error | null) => void): void;
}): Promise<void> {
	await new Promise<void>((resolve, reject) => {
		server.close((error?: Error | null) => {
			if (error) {
				reject(error);
				return;
			}

			resolve();
		});
	});
}

async function waitForServer(server: {
	listening?: boolean;
	once(event: "error", listener: (error: Error) => void): void;
	once(event: "listening", listener: () => void): void;
}): Promise<void> {
	if (server.listening) {
		return;
	}

	await new Promise<void>((resolve, reject) => {
		server.once("error", reject);
		server.once("listening", resolve);
	});
}
