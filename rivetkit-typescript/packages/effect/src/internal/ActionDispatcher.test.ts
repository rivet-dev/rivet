import { assert, describe, it } from "@effect/vitest";
import { Action, Actor } from "@rivetkit/effect";
import * as Rivetkit from "rivetkit";
import * as ActionDispatcher from "./ActionDispatcher.ts";

const Ping = Action.make("Ping");

const dispatchWithoutInstance = (abortSignal: AbortSignal) => {
	const handlers = ActionDispatcher.make({
		actor: Actor.make("Probe", { actions: [Ping] }),
		getInstance: () => undefined,
	});
	const c = {
		actorId: "probe",
		abortSignal,
	} as Rivetkit.ActionContextOf<Rivetkit.AnyActorDefinition>;
	return handlers.Ping(c, undefined);
};

const assertActorAborted = async (dispatch: Promise<unknown>) => {
	const error = await dispatch.then(
		() => assert.fail("expected the action to be rejected"),
		(error: unknown) => error,
	);
	assert.instanceOf(error, Rivetkit.RivetError);
	assert.strictEqual(error.group, "actor");
	assert.strictEqual(error.code, "aborted");
};

describe("internal/ActionDispatcher", () => {
	it("rejects an action that reaches a torn-down instance before its signal aborts", async () => {
		await assertActorAborted(
			dispatchWithoutInstance(new AbortController().signal),
		);
	});

	it("rejects an action that reaches a torn-down instance after its signal aborts", async () => {
		await assertActorAborted(dispatchWithoutInstance(AbortSignal.abort()));
	});
});
