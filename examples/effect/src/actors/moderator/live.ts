import { State } from "@rivetkit/effect";
import { Effect, Schema } from "effect";
import { Moderator } from "./api.ts";

export const ModeratorLive = Moderator.toLayer(
	({ state }) =>
		Effect.gen(function* () {
			return Moderator.of({
				Review: ({ payload }) =>
					Effect.gen(function* () {
						// State writes go through Effect Schema validation. This
						// example treats schema failures as defects instead of adding
						// typed error channels to the action contract.
						const next = yield* State.updateAndGet(
							state,
							(current) => ({
								...current,
								reviewed: current.reviewed + 1,
							}),
						).pipe(Effect.orDie);
						const lower = payload.text.toLowerCase();
						const hit = next.bannedWords.find((word) =>
							lower.includes(word),
						);

						return hit
							? {
									approved: false,
									reason: `contains banned word "${hit}"`,
								}
							: { approved: true };
					}),
				Stats: () =>
					State.get(state).pipe(
						Effect.orDie,
						Effect.map(({ reviewed }) => ({ reviewed })),
					),
			});
		}),
	{
		state: {
			schema: Schema.Struct({
				bannedWords: Schema.Array(Schema.String),
				reviewed: Schema.Number,
			}),
			initialValue: () => ({
				bannedWords: ["spam", "scam"],
				reviewed: 0,
			}),
		},
		name: "Moderator",
		icon: "shield",
	},
);
