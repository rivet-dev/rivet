import { Effect } from "effect";
import { db } from "rivetkit/db";
import { BannedWordsError, Moderator } from "./api.ts";

const bannedWords = ["spam", "scam"];

export const ModeratorLive = Moderator.toLayer(
	Effect.fnUntraced(function* ({ rawRivetkitContext }) {
		return Moderator.of({
			Review: Effect.fnUntraced(function* ({ payload }) {
				// Count every reviewed message in the actor's SQLite database.
				yield* Effect.tryPromise(() =>
					rawRivetkitContext.db.execute(
						"UPDATE stats SET reviewed = reviewed + 1 WHERE id = 1",
					),
				).pipe(Effect.orDie);

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
		db: db({
			onMigrate: async (client) => {
				await client.execute(`
					CREATE TABLE IF NOT EXISTS stats (
						id INTEGER PRIMARY KEY CHECK (id = 1),
						reviewed INTEGER NOT NULL
					)
				`);
				await client.execute(
					"INSERT OR IGNORE INTO stats (id, reviewed) VALUES (1, 0)",
				);
			},
		}),
		name: "Moderator",
		icon: "shield",
	},
);
