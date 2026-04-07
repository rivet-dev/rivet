import { describe, expect, test } from "vitest";
import {
	resolveGatewayTarget,
	type ActorOutput,
	type GatewayTarget,
	type EngineControlClient,
} from "@/driver-helpers/mod";

describe("resolveGatewayTarget", () => {
	test("passes through direct actor IDs", async () => {
		const driver = createMockDriver();

		await expect(
			resolveGatewayTarget(driver, { directId: "direct-actor-id" }),
		).resolves.toBe("direct-actor-id");
	});

	test("resolves getForKey targets and reports missing actors", async () => {
		const driver = createMockDriver({
			getWithKey: async ({ key }) =>
				key[0] === "room"
					? actorOutput("resolved-key-actor")
					: undefined,
		});

		await expect(
			resolveGatewayTarget(driver, {
				getForKey: {
					name: "counter",
					key: ["room"],
				},
			}),
		).resolves.toBe("resolved-key-actor");

		await expect(
			resolveGatewayTarget(driver, {
				getForKey: {
					name: "counter",
					key: ["missing"],
				},
			}),
		).rejects.toMatchObject({
			group: "actor",
			code: "not_found",
		});
	});

	test("forwards create and getOrCreate inputs", async () => {
		const getOrCreateCalls: Array<Record<string, unknown>> = [];
		const createCalls: Array<Record<string, unknown>> = [];
		const driver = createMockDriver({
			getOrCreateWithKey: async (input) => {
				getOrCreateCalls.push(
					input as unknown as Record<string, unknown>,
				);
				return actorOutput("get-or-create-actor");
			},
			createActor: async (input) => {
				createCalls.push(input as unknown as Record<string, unknown>);
				return actorOutput("created-actor");
			},
		});

		await expect(
			resolveGatewayTarget(driver, {
				getOrCreateForKey: {
					name: "counter",
					key: ["room"],
					input: { ready: true },
					region: "iad",
				},
			}),
		).resolves.toBe("get-or-create-actor");

		await expect(
			resolveGatewayTarget(driver, {
				create: {
					name: "counter",
					key: ["room"],
					input: { ready: true },
					region: "sfo",
				},
			}),
		).resolves.toBe("created-actor");

		expect(getOrCreateCalls).toEqual([
			expect.objectContaining({
				name: "counter",
				key: ["room"],
				input: { ready: true },
				region: "iad",
			}),
		]);
		expect(createCalls).toEqual([
			expect.objectContaining({
				name: "counter",
				key: ["room"],
				input: { ready: true },
				region: "sfo",
			}),
		]);
	});

	test("rejects invalid target shapes", async () => {
		const driver = createMockDriver();

		await expect(
			resolveGatewayTarget(driver, {} as GatewayTarget),
		).rejects.toMatchObject({
			group: "request",
			code: "invalid",
		});
	});
});

function createMockDriver(
	overrides: Partial<EngineControlClient> = {},
): EngineControlClient {
	return {
		getForId: async () => undefined,
		getWithKey: async () => undefined,
		getOrCreateWithKey: async () => actorOutput("get-or-create-default"),
		createActor: async () => actorOutput("create-default"),
		listActors: async () => [],
		sendRequest: async () => {
			throw new Error("sendRequest not implemented in test");
		},
		openWebSocket: async () => {
			throw new Error("openWebSocket not implemented in test");
		},
		proxyRequest: async () => {
			throw new Error("proxyRequest not implemented in test");
		},
		proxyWebSocket: async () => {
			throw new Error("proxyWebSocket not implemented in test");
		},
		buildGatewayUrl: async () => {
			throw new Error("buildGatewayUrl not implemented in test");
		},
		displayInformation: () => ({ properties: {} }),
		setGetUpgradeWebSocket: () => {},
		kvGet: async () => null,
		kvBatchGet: async (_actorId, keys) => keys.map(() => null),
		kvBatchPut: async () => {},
		kvBatchDelete: async () => {},
		kvDeleteRange: async () => {},
		...overrides,
	};
}

function actorOutput(actorId: string): ActorOutput {
	return {
		actorId,
		name: "counter",
		key: [],
	};
}
