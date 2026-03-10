import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { createClient } from "@/client/mod";
import { createTestRuntime } from "@/driver-test-suite/mod";
import { createEngineDriver } from "@/drivers/engine/mod";
import invariant from "invariant";
import { createClientWithDriver } from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { afterEach, describe, expect, test } from "vitest";
import { DYNAMIC_SOURCE } from "../fixtures/driver-test-suite/dynamic-registry";
import type { registry } from "../fixtures/driver-test-suite/dynamic-registry";

const SECURE_EXEC_DIST_PATH = join(
	process.env.HOME ?? "",
	"secure-exec-rivet/packages/secure-exec/dist/index.js",
);
const hasSecureExecDist = existsSync(SECURE_EXEC_DIST_PATH);
const hasEngineEndpointEnv = !!(
	process.env.RIVET_ENDPOINT ||
	process.env.RIVET_NAMESPACE_ENDPOINT ||
	process.env.RIVET_API_ENDPOINT
);
const initialDynamicSourceUrlEnv =
	process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL;
const initialSecureExecSpecifierEnv =
	process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER;

type DynamicHandle = {
	increment: (amount?: number) => Promise<number>;
	getSourceCodeLength: () => Promise<number>;
	getState: () => Promise<{
		count: number;
		wakeCount: number;
		sleepCount: number;
		alarmCount: number;
	}>;
	putText: (key: string, value: string) => Promise<boolean>;
	getText: (key: string) => Promise<string | null>;
	listText: (prefix: string) => Promise<Array<{ key: string; value: string }>>;
	triggerSleep: () => Promise<boolean>;
	scheduleAlarm: (duration: number) => Promise<boolean>;
	webSocket: (path?: string) => Promise<WebSocket>;
};

type DynamicAuthHandle = DynamicHandle & {
	fetch: (input: string | URL | Request, init?: RequestInit) => Promise<Response>;
};

describe.skipIf(!hasSecureExecDist || !hasEngineEndpointEnv)(
	"engine dynamic actor runtime",
	() => {
	let sourceServer:
		| {
				url: string;
				close: () => Promise<void>;
		  }
		| undefined;

	afterEach(async () => {
		if (sourceServer) {
			await sourceServer.close();
			sourceServer = undefined;
		}
		if (initialDynamicSourceUrlEnv === undefined) {
			delete process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL;
		} else {
			process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL =
				initialDynamicSourceUrlEnv;
		}
		if (initialSecureExecSpecifierEnv === undefined) {
			delete process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER;
		} else {
			process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER =
				initialSecureExecSpecifierEnv;
		}
	});

	test("loads dynamic actor source from URL", async () => {
		sourceServer = await startSourceServer(DYNAMIC_SOURCE);
		process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL = sourceServer.url;
		process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER = pathToFileURL(
			SECURE_EXEC_DIST_PATH,
		).href;

		const runtime = await createDynamicEngineRuntime();
		const client = createClient<typeof registry>({
			endpoint: runtime.endpoint,
			namespace: runtime.namespace,
			runnerName: runtime.runnerName,
			encoding: "json",
			disableMetadataLookup: true,
		});
		const bareClient = createClient<typeof registry>({
			endpoint: runtime.endpoint,
			namespace: runtime.namespace,
			runnerName: runtime.runnerName,
			encoding: "bare",
			disableMetadataLookup: true,
		});

		try {
			const actor = client.dynamicFromUrl.getOrCreate([
				"url-loader",
			]) as unknown as DynamicHandle;
			expect(await actor.increment(2)).toBe(2);
			expect(await actor.increment(3)).toBe(5);
			expect(await actor.getSourceCodeLength()).toBeGreaterThan(0);

			const bareActor = bareClient.dynamicFromUrl.getOrCreate([
				"url-loader",
			]) as unknown as DynamicHandle;
			expect(await bareActor.increment(1)).toBe(6);

			const state = await actor.getState();
			expect(state.count).toBe(6);
			expect(state.wakeCount).toBeGreaterThanOrEqual(1);
		} finally {
			await client.dispose();
			await bareClient.dispose();
			await runtime.cleanup();
		}
	}, 180_000);

	test("supports actions, kv, websockets, alarms, and sleep/wake from actor-loaded source", async () => {
		sourceServer = await startSourceServer(DYNAMIC_SOURCE);
		process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL = sourceServer.url;
		process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER = pathToFileURL(
			SECURE_EXEC_DIST_PATH,
		).href;

		const runtime = await createDynamicEngineRuntime();
		const client = createClient<typeof registry>({
			endpoint: runtime.endpoint,
			namespace: runtime.namespace,
			runnerName: runtime.runnerName,
			encoding: "json",
			disableMetadataLookup: true,
		});

		let ws: WebSocket | undefined;

		try {
			const actor = client.dynamicFromActor.getOrCreate([
				"actor-loader",
			]) as unknown as DynamicHandle;

			expect(await actor.increment(1)).toBe(1);
			expect(await actor.getSourceCodeLength()).toBeGreaterThan(0);

			await actor.putText("prefix-a", "alpha");
			await actor.putText("prefix-b", "beta");
			expect(await actor.getText("prefix-a")).toBe("alpha");
			expect(
				(await actor.listText("prefix-")).sort((a, b) =>
					a.key.localeCompare(b.key),
				),
			).toEqual([
				{ key: "prefix-a", value: "alpha" },
				{ key: "prefix-b", value: "beta" },
			]);

			ws = await actor.webSocket();
			const welcome = await readWebSocketJson(ws);
			expect(welcome).toMatchObject({ type: "welcome" });
			ws.send(JSON.stringify({ type: "ping" }));
			expect(await readWebSocketJson(ws)).toEqual({ type: "pong" });
			ws.close();
			ws = undefined;

			const beforeSleep = await actor.getState();
			await actor.triggerSleep();
			await wait(350);

			const afterSleep = await actor.getState();
			expect(afterSleep.sleepCount).toBeGreaterThanOrEqual(
				beforeSleep.sleepCount + 1,
			);
			expect(afterSleep.wakeCount).toBeGreaterThanOrEqual(
				beforeSleep.wakeCount + 1,
			);

			const beforeAlarm = await actor.getState();
			await actor.scheduleAlarm(500);
			await wait(900);

			const afterAlarm = await actor.getState();
			expect(afterAlarm.alarmCount).toBeGreaterThanOrEqual(
				beforeAlarm.alarmCount + 1,
			);
			expect(afterAlarm.sleepCount).toBeGreaterThanOrEqual(
				beforeAlarm.sleepCount + 1,
			);
			expect(afterAlarm.wakeCount).toBeGreaterThanOrEqual(
				beforeAlarm.wakeCount + 1,
			);
		} finally {
			ws?.close();
			await client.dispose();
			await runtime.cleanup();
		}
	}, 180_000);

	test("authenticates dynamic actor actions, raw requests, and websockets", async () => {
		sourceServer = await startSourceServer(DYNAMIC_SOURCE);
		process.env.RIVETKIT_DYNAMIC_TEST_SOURCE_URL = sourceServer.url;
		process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER = pathToFileURL(
			SECURE_EXEC_DIST_PATH,
		).href;

		const runtime = await createDynamicEngineRuntime();
		const client = createClient<typeof registry>({
			endpoint: runtime.endpoint,
			namespace: runtime.namespace,
			runnerName: runtime.runnerName,
			encoding: "json",
			disableMetadataLookup: true,
		});

		let ws: WebSocket | undefined;

		try {
			const unauthorized = client.dynamicWithAuth.getOrCreate([
				"auth-unauthorized",
			]) as unknown as DynamicAuthHandle;
			await expect(unauthorized.increment(1)).rejects.toMatchObject({
				group: "user",
				code: "unauthorized",
			});

			const unauthorizedResponse = await unauthorized.fetch("/auth");
			expect(unauthorizedResponse.status).toBe(400);
			expect(await unauthorizedResponse.json()).toMatchObject({
				group: "user",
				code: "unauthorized",
			});

			const headerAuthorized = client.dynamicWithAuth.getOrCreate([
				"auth-header",
			]) as unknown as DynamicAuthHandle;
			const headerResponse = await headerAuthorized.fetch("/auth", {
				headers: {
					"x-dynamic-auth": "allow",
				},
			});
			expect(headerResponse.status).toBe(200);
			expect(await headerResponse.json()).toEqual({
				method: "GET",
				token: "allow",
			});

			const paramsAuthorized = client.dynamicWithAuth.getOrCreate(
				["auth-params"],
				{
					params: {
						token: "allow",
					},
				},
			) as unknown as DynamicAuthHandle;
			expect(await paramsAuthorized.increment(1)).toBe(1);

			ws = await paramsAuthorized.webSocket();
			expect(await readWebSocketJson(ws)).toMatchObject({
				type: "welcome",
			});
		} finally {
			ws?.close();
			await client.dispose();
			await runtime.cleanup();
		}
	}, 180_000);
	},
);

async function createDynamicEngineRuntime() {
	return await createTestRuntime(
		join(__dirname, "../fixtures/driver-test-suite/dynamic-registry.ts"),
		async (registry) => {
			const endpoint = process.env.RIVET_ENDPOINT || "http://127.0.0.1:6420";
			const namespaceEndpoint =
				process.env.RIVET_NAMESPACE_ENDPOINT ||
				process.env.RIVET_API_ENDPOINT ||
				endpoint;
			const namespace = `test-${crypto.randomUUID().slice(0, 8)}`;
			const runnerName = "test-runner";
			const token = "dev";

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

			const driverConfig = createEngineDriver();
			registry.config.driver = driverConfig;
			registry.config.endpoint = endpoint;
			registry.config.namespace = namespace;
			registry.config.token = token;
			registry.config.runner = {
				...registry.config.runner,
				runnerName,
			};

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

			const runnersUrl = new URL(`${endpoint.replace(/\/$/, "")}/runners`);
			runnersUrl.searchParams.set("namespace", namespace);
			runnersUrl.searchParams.set("name", runnerName);
			let probeError: unknown;
			for (let attempt = 0; attempt < 120; attempt++) {
				try {
					const runnerResponse = await fetch(runnersUrl, {
						method: "GET",
						headers: { Authorization: `Bearer ${token}` },
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
}

async function startSourceServer(source: string): Promise<{
	url: string;
	close: () => Promise<void>;
}> {
	const server = createServer((req: IncomingMessage, res: ServerResponse) => {
		if (req.url !== "/source.ts") {
			res.writeHead(404);
			res.end("not found");
			return;
		}

		res.writeHead(200, {
			"content-type": "text/plain; charset=utf-8",
		});
		res.end(source);
	});

	await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
	const address = server.address();
	if (!address || typeof address === "string") {
		throw new Error("failed to get dynamic source server address");
	}

	return {
		url: `http://127.0.0.1:${address.port}/source.ts`,
		close: async () => {
			await new Promise<void>((resolve, reject) => {
				server.close((error) => {
					if (error) {
						reject(error);
						return;
					}
					resolve();
				});
			});
		},
	};
}

async function readWebSocketJson(websocket: WebSocket): Promise<any> {
	const message = await new Promise<string>((resolve, reject) => {
		const timeout = setTimeout(() => {
			reject(new Error("timed out waiting for websocket message"));
		}, 5_000);

		websocket.addEventListener(
			"message",
			(event) => {
				clearTimeout(timeout);
				resolve(String(event.data));
			},
			{ once: true },
		);
		websocket.addEventListener(
			"error",
			(event: Event) => {
				clearTimeout(timeout);
				reject(event);
			},
			{ once: true },
		);
		websocket.addEventListener(
			"close",
			() => {
				clearTimeout(timeout);
				reject(new Error("websocket closed"));
			},
			{ once: true },
		);
	});

	return JSON.parse(message);
}

async function wait(duration: number): Promise<void> {
	await new Promise((resolve) => setTimeout(resolve, duration));
}
