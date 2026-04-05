import { describe, expect, test, vi } from "vitest";
import { ClientRaw } from "@/client/client";
import type {
	ActorOutput,
	GatewayTarget,
	ManagerDriver,
} from "@/driver-helpers/mod";
import { PATH_CONNECT } from "@/driver-helpers/mod";

describe("actor resolution flow", () => {
	test("get handles resolve a fresh actor ID on each operation", async () => {
		const getWithKeyCalls: string[] = [];
		const driver = createMockDriver({
			getWithKey: async () => {
				const actorId = `get-actor-${getWithKeyCalls.length + 1}`;
				getWithKeyCalls.push(actorId);
				return actorOutput(actorId);
			},
		});
		const client = new ClientRaw(driver, undefined);
		const handle = client.get("counter", ["room"]);

		expect(await handle.resolve()).toBe("get-actor-1");
		expect(await handle.resolve()).toBe("get-actor-2");
		expect(getWithKeyCalls).toEqual(["get-actor-1", "get-actor-2"]);
	});

	test("get handles pass ActorQuery targets through gateway operations", async () => {
		const expectedTarget = {
			getForKey: {
				name: "counter",
				key: ["room"],
			},
		} satisfies GatewayTarget;
		const sendTargets: GatewayTarget[] = [];
		const gatewayTargets: GatewayTarget[] = [];
		const webSocketCalls: Array<{
			path: string;
			target: GatewayTarget;
			socket: MockWebSocket;
		}> = [];
		const driver = createMockDriver({
			sendRequest: async (target, actorRequest) => {
				sendTargets.push(target);
				const pathname = new URL(actorRequest.url).pathname;
				if (pathname.endsWith("/action/ping")) {
					return Response.json({ output: "pong" });
				}

				return new Response("ok");
			},
			openWebSocket: async (path, target) => {
				const socket = new MockWebSocket();
				webSocketCalls.push({ path, target, socket });

				if (path === PATH_CONNECT) {
					setTimeout(() => {
						socket.emitOpen();
						socket.emitMessage(
							JSON.stringify({
								body: {
									tag: "Init",
									val: {
										actorId: "query-actor",
										connectionId: "conn-1",
									},
								},
							}),
						);
					}, 0);
				}

				return socket as any;
			},
			buildGatewayUrl: async (target) => {
				gatewayTargets.push(target);
				return "gateway:query";
			},
		});
		const client = new ClientRaw(driver, "json");
		const handle = client.get("counter", ["room"]);

		expect(await handle.action({ name: "ping", args: [] })).toBe("pong");
		expect(await (await handle.fetch("/resource")).text()).toBe("ok");
		await handle.webSocket("/stream");

		const conn = handle.connect();
		await vi.waitFor(() => {
			expect(conn.connStatus).toBe("connected");
		});

		expect(await handle.getGatewayUrl()).toBe("gateway:query");
		expect(sendTargets).toEqual([expectedTarget, expectedTarget]);
		expect(gatewayTargets).toEqual([expectedTarget]);
		expect(webSocketCalls).toHaveLength(2);
		expect(webSocketCalls[0]?.target).toEqual(expectedTarget);
		expect(webSocketCalls[1]?.target).toEqual(expectedTarget);
		expect(webSocketCalls[1]?.path).toBe(PATH_CONNECT);

		await conn.dispose();
	});

	test("getOrCreate handles build query gateway URLs without resolving actor IDs", async () => {
		const expectedTarget = {
			getOrCreateForKey: {
				name: "counter",
				key: ["room"],
				input: undefined,
				region: undefined,
			},
		} satisfies GatewayTarget;
		let getOrCreateCalls = 0;
		const gatewayTargets: GatewayTarget[] = [];
		const driver = createMockDriver({
			getOrCreateWithKey: async () => {
				getOrCreateCalls += 1;
				return actorOutput(`get-or-create-${getOrCreateCalls}`);
			},
			buildGatewayUrl: async (target) => {
				gatewayTargets.push(target);
				return "gateway:query";
			},
		});
		const client = new ClientRaw(driver, undefined);
		const handle = client.getOrCreate("counter", ["room"]);

		expect(await handle.resolve()).toBe("get-or-create-1");
		expect(await handle.resolve()).toBe("get-or-create-2");
		expect(await handle.getGatewayUrl()).toBe("gateway:query");
		expect(getOrCreateCalls).toBe(2);
		expect(gatewayTargets).toEqual([expectedTarget]);
	});

	test("query-backed connections reconnect with ActorQuery targets", async () => {
		const expectedTarget = {
			getOrCreateForKey: {
				name: "counter",
				key: ["room"],
				input: undefined,
				region: undefined,
			},
		} satisfies GatewayTarget;
		const webSocketCalls: Array<{
			target: GatewayTarget;
			socket: MockWebSocket;
		}> = [];
		const driver = createMockDriver({
			openWebSocket: async (path, target) => {
				expect(path).toBe(PATH_CONNECT);

				const socket = new MockWebSocket();
				webSocketCalls.push({ target, socket });

				setTimeout(() => {
					socket.emitOpen();
					socket.emitMessage(
						JSON.stringify({
							body: {
								tag: "Init",
								val: {
									actorId: `actor-${webSocketCalls.length}`,
									connectionId: `conn-${webSocketCalls.length}`,
								},
							},
						}),
					);
				}, 0);

				return socket as any;
			},
		});
		const client = new ClientRaw(driver, "json");
		const conn = client.getOrCreate("counter", ["room"]).connect();

		await vi.waitFor(() => {
			expect(conn.connStatus).toBe("connected");
		});

		webSocketCalls[0]?.socket.emitClose({
			code: 1011,
			reason: "connection_lost",
			wasClean: false,
		});

		await vi.waitFor(() => {
			expect(webSocketCalls).toHaveLength(2);
		});
		await vi.waitFor(() => {
			expect(conn.connStatus).toBe("connected");
		});
		expect(webSocketCalls.map((call) => call.target)).toEqual([
			expectedTarget,
			expectedTarget,
		]);

		await conn.dispose();
	});

	test("getForId handles keep their explicit actor ID for gateway calls", async () => {
		let getForIdCalls = 0;
		const sendTargets: GatewayTarget[] = [];
		const gatewayTargets: GatewayTarget[] = [];
		const webSocketCalls: Array<{
			path: string;
			target: GatewayTarget;
			socket: MockWebSocket;
		}> = [];
		const driver = createMockDriver({
			getForId: async () => {
				getForIdCalls += 1;
				return actorOutput("manager-looked-up");
			},
			sendRequest: async (target, actorRequest) => {
				sendTargets.push(target);
				const pathname = new URL(actorRequest.url).pathname;
				if (pathname.endsWith("/action/ping")) {
					return Response.json({ output: "pong" });
				}

				return new Response("ok");
			},
			openWebSocket: async (path, target) => {
				const socket = new MockWebSocket();
				webSocketCalls.push({ path, target, socket });

				if (path === PATH_CONNECT) {
					setTimeout(() => {
						socket.emitOpen();
						socket.emitMessage(
							JSON.stringify({
								body: {
									tag: "Init",
									val: {
										actorId: "explicit-actor",
										connectionId: "conn-1",
									},
								},
							}),
						);
					}, 0);
				}

				return socket as any;
			},
			buildGatewayUrl: async (target) => {
				gatewayTargets.push(target);
				return `gateway:${describeGatewayTarget(target)}`;
			},
		});
		const client = new ClientRaw(driver, "json");
		const handle = client.getForId("counter", "explicit-actor");

		const expectedDirectTarget = { directId: "explicit-actor" };
		expect(await handle.action({ name: "ping", args: [] })).toBe("pong");
		expect(await (await handle.fetch("/resource")).text()).toBe("ok");
		await handle.webSocket("/stream");
		expect(await handle.resolve()).toBe("explicit-actor");
		expect(await handle.getGatewayUrl()).toBe("gateway:explicit-actor");
		const conn = handle.connect();
		await vi.waitFor(() => {
			expect(conn.connStatus).toBe("connected");
		});
		expect(sendTargets).toEqual([expectedDirectTarget, expectedDirectTarget]);
		expect(gatewayTargets).toEqual([expectedDirectTarget]);
		expect(webSocketCalls).toHaveLength(2);
		expect(webSocketCalls[0]?.target).toEqual(expectedDirectTarget);
		expect(webSocketCalls[1]?.target).toEqual(expectedDirectTarget);
		expect(getForIdCalls).toBe(0);

		await conn.dispose();
	});

	test("create returns a handle pinned to the created actor ID", async () => {
		let createCalls = 0;
		let getForIdCalls = 0;
		const driver = createMockDriver({
			createActor: async () => {
				createCalls += 1;
				return actorOutput("created-actor");
			},
			getForId: async () => {
				getForIdCalls += 1;
				return actorOutput("manager-looked-up");
			},
		});
		const client = new ClientRaw(driver, undefined);
		const handle = await client.create("counter", ["room"]);

		expect(await handle.resolve()).toBe("created-actor");
		expect(await handle.getGatewayUrl()).toBe("gateway:created-actor");
		expect(createCalls).toBe(1);
		expect(getForIdCalls).toBe(0);
	});
});

function createMockDriver(overrides: Partial<ManagerDriver>): ManagerDriver {
	return {
		getForId: async () => undefined,
		getWithKey: async () => undefined,
		getOrCreateWithKey: async ({ name, key }) =>
			actorOutput(`${name}:${key.join(",")}`),
		createActor: async ({ name, key }) =>
			actorOutput(`created:${name}:${key.join(",")}`),
		listActors: async () => [],
		sendRequest: async (_target: GatewayTarget, _actorRequest: Request) => {
			throw new Error("sendRequest should not be called in this test");
		},
		openWebSocket: async () => {
			throw new Error("openWebSocket should not be called in this test");
		},
		proxyRequest: async () => {
			throw new Error("proxyRequest should not be called in this test");
		},
		proxyWebSocket: async () => {
			throw new Error("proxyWebSocket should not be called in this test");
		},
		buildGatewayUrl: async (target: GatewayTarget) =>
			`gateway:${describeGatewayTarget(target)}`,
		displayInformation: () => ({ properties: {} }),
		setGetUpgradeWebSocket: () => {},
		kvGet: async () => null,
		...overrides,
	};
}

function describeGatewayTarget(target: GatewayTarget): string {
	if ("directId" in target) {
		return target.directId;
	}

	if ("getForId" in target) {
		return `query:getForId:${target.getForId.actorId}`;
	}

	if ("getForKey" in target) {
		return `query:get:${target.getForKey.name}:${target.getForKey.key.join(",")}`;
	}

	if ("getOrCreateForKey" in target) {
		return `query:getOrCreate:${target.getOrCreateForKey.name}:${target.getOrCreateForKey.key.join(",")}`;
	}

	return `query:create:${target.create.name}:${target.create.key.join(",")}`;
}

function actorOutput(actorId: string): ActorOutput {
	return {
		actorId,
		name: "counter",
		key: [],
	};
}

class MockWebSocket {
	readyState = 1;
	#listeners = new Map<string, Set<(event: any) => void>>();

	addEventListener(type: string, listener: (event: any) => void) {
		let listeners = this.#listeners.get(type);
		if (!listeners) {
			listeners = new Set();
			this.#listeners.set(type, listeners);
		}

		listeners.add(listener);
	}

	removeEventListener(type: string, listener: (event: any) => void) {
		this.#listeners.get(type)?.delete(listener);
	}

	send(_data: unknown) {}

	close(code = 1000, reason = "") {
		this.emitClose({
			code,
			reason,
			wasClean: code === 1000,
		});
	}

	emitOpen() {
		this.readyState = 1;
		this.#emit("open", {});
	}

	emitMessage(data: string) {
		this.#emit("message", { data });
	}

	emitClose(event: { code: number; reason: string; wasClean: boolean }) {
		this.readyState = 3;
		this.#emit("close", event);
	}

	#emit(type: string, event: any) {
		for (const listener of this.#listeners.get(type) ?? []) {
			listener(event);
		}
	}
}
