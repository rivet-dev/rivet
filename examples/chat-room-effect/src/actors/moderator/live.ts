import { Effect, Schema } from "effect";
import { BannedWordsError, Moderator } from "./api.ts";

const bannedWords = ["spam", "scam"];

export const ModeratorLive = Moderator.toLayer(
	Effect.fnUntraced(function* ({ state }) {
		return Moderator.of({
			Review: Effect.fnUntraced(function* ({ payload }) {
				yield* state
					.update((current) => ({
						...current,
						reviewed: current.reviewed + 1,
					}))
					.pipe(Effect.orDie);

				const lower = payload.text.toLowerCase();
				const hit = bannedWords.find((word) => lower.includes(word));
				if (hit !== undefined) {
					return yield* new BannedWordsError({
						message: `contains banned word "${hit}"`,
					});
				}
			}),
		});
	}),
	{
		state: {
			schema: Schema.Struct({
				reviewed: Schema.Number,
			}),
			initialValue: () => ({
				reviewed: 0,
			}),
		},
		name: "Moderator",
		icon: "shield",
	},
);
