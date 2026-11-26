import { Action, Actor } from "@rivetkit/effect";
import { Effect, Ref, Schema } from "effect";

// docs:start contract
export const Hit = Action.make("Hit", { success: Schema.Number });

export const HitCounter = Actor.make("HitCounter", {
	actions: [Hit],
});
// docs:end contract

// docs:start implementation
export const HitCounterLive = HitCounter.toLayer(
	Effect.gen(function* () {
		// In-memory data. The actor loses it when it sleeps.
		const hits = yield* Ref.make(0);

		yield* Effect.addFinalizer(() =>
			Ref.get(hits).pipe(
				Effect.flatMap((count) =>
					Effect.log(`Sleeping after ${count} hits`),
				),
			),
		);

		return HitCounter.of({
			Hit: () => Ref.updateAndGet(hits, (count) => count + 1),
		});
	}),
);
// docs:end implementation
