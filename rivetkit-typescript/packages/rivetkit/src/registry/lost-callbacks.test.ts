import { EvictedError } from "@rivetkit/workflow-engine";
import { describe, expect, test } from "vitest";
import { ACTOR_CONTEXT_INTERNAL_SYMBOL } from "@/actor/config";
import { throwIfGenerationLost } from "@/workflow/context";
import { refuseCallbacksAfterLost } from "./native";
import type { CoreRuntime } from "./runtime";

function fakeRuntime(isLost: () => boolean): CoreRuntime {
	return { actorIsLost: () => isLost() } as unknown as CoreRuntime;
}

describe("lost generation callback gate", () => {
	test("refuses runtime callbacks once the generation is lost", async () => {
		let lost = false;
		const guarded = refuseCallbacksAfterLost(
			fakeRuntime(() => lost),
			{
				createVars: async (_error: unknown, _payload: unknown) =>
					"createVars ran",
				onSleep: async (_error: unknown, _payload: unknown) =>
					"cleanup ran",
				onDestroy: async (_error: unknown, _payload: unknown) =>
					"destroy cleanup ran",
				actions: {
					increment: async (_error: unknown, _payload: unknown) =>
						"action ran",
				},
				hasState: true,
				onRequest: undefined,
			},
		);
		const payload = { ctx: {} };

		await expect(guarded.createVars(null, payload)).resolves.toBe(
			"createVars ran",
		);
		await expect(guarded.actions.increment(null, payload)).resolves.toBe(
			"action ran",
		);

		lost = true;
		await expect(guarded.createVars(null, payload)).rejects.toBeDefined();
		await expect(
			guarded.actions.increment(null, payload),
		).rejects.toBeDefined();
		// Cleanup callbacks still run so the generation releases its runtime state.
		await expect(guarded.onSleep(null, payload)).resolves.toBe(
			"cleanup ran",
		);
		await expect(guarded.onDestroy(null, payload)).resolves.toBe(
			"destroy cleanup ran",
		);
		expect(guarded.hasState).toBe(true);
		expect(guarded.onRequest).toBeUndefined();
	});

	test("workflow code stops before more user callbacks once lost", () => {
		const actor = { isLost: false };
		const runCtx = { [ACTOR_CONTEXT_INTERNAL_SYMBOL]: actor };
		expect(() => throwIfGenerationLost(runCtx)).not.toThrow();
		actor.isLost = true;
		expect(() => throwIfGenerationLost(runCtx)).toThrow(EvictedError);
	});
});
