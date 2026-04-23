import { actor, setup } from "@/mod";
import { RivetError } from "../src/actor/errors";
import {
	NativeActorContextAdapter,
	buildNativeRegistry,
} from "../src/registry/native";
import { describe, expect, test } from "vitest";

const testActor = actor({
	state: {},
	actions: {},
});

function createNativeActorContextAdapter(options?: {
	stateEnabled?: boolean;
}): NativeActorContextAdapter {
	return new NativeActorContextAdapter(
		{} as never,
		{
			actorId: () => "actor-id",
		} as never,
		undefined,
		{},
		undefined,
		undefined,
		options?.stateEnabled ?? true,
	);
}

function captureThrownError(run: () => unknown): unknown {
	try {
		run();
	} catch (error) {
		return error;
	}

	throw new Error("expected function to throw");
}

describe("native runtime config errors", () => {
	test("ctx.client preserves structured error fields when client is missing", () => {
		const actorCtx = createNativeActorContextAdapter();
		const error = captureThrownError(() => actorCtx.client());

		expect(error).toBeInstanceOf(RivetError);
		expect(error).toMatchObject({
			group: "native",
			code: "client_not_configured",
			message: "native actor client is not configured",
		});
	});

	test("ctx.db preserves structured error fields when database is missing", () => {
		const actorCtx = createNativeActorContextAdapter();
		const error = captureThrownError(() => actorCtx.db);

		expect(error).toBeInstanceOf(RivetError);
		expect(error).toMatchObject({
			group: "actor",
			code: "database_not_configured",
			message: "database is not configured for this actor",
		});
	});

	test("ctx.state preserves structured error fields when state is disabled", () => {
		const actorCtx = createNativeActorContextAdapter({
			stateEnabled: false,
		});
		const error = captureThrownError(() => actorCtx.state);

		expect(error).toBeInstanceOf(RivetError);
		expect(error).toMatchObject({
			group: "actor",
			code: "state_not_enabled",
			message:
				"State not enabled. Must implement `createState` or `state` to use state. (https://www.rivet.dev/docs/actors/state/#initializing-state)",
		});
	});

	test("buildNativeRegistry preserves structured error fields when endpoint is missing", async () => {
		const registry = setup({
			use: {
				test: testActor,
			},
			startEngine: false,
		});
		const config = registry.parseConfig();
		config.endpoint = undefined as never;

		await expect(buildNativeRegistry(config)).rejects.toMatchObject({
			group: "native",
			code: "endpoint_not_configured",
			message: "registry endpoint is required for native envoy startup",
		});
	});
});
