import { Effect } from "effect";
import { Workspace } from "./workspace";

export const program = Effect.gen(function* () {
	const workspaces = yield* Workspace.client;

	const acme = yield* workspaces.create("acme", {
		input: { owner: "alice" },
	});
	yield* Effect.log(`Owner of acme: ${yield* acme.GetOwner()}`);

	// The actor gets `createWithInput` only if this call creates it.
	const globex = workspaces.getOrCreate("globex", {
		createWithInput: { owner: "bob" },
	});
	yield* Effect.log(`Owner of globex: ${yield* globex.GetOwner()}`);
});
