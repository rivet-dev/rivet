import { actor, event } from "rivetkit";
import { db } from "rivetkit/db";
import { DEFAULT_RATING } from "./config.ts";

export const rankedPlayer = actor({
	options: { name: "Ranked - Player", icon: "ranking-star" },
	events: {
		stateUpdate: event<PlayerSnapshot>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS profile (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					username TEXT NOT NULL,
					rating INTEGER NOT NULL,
					wins INTEGER NOT NULL,
					losses INTEGER NOT NULL
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO profile (id, username, rating, wins, losses) VALUES (1, '', ?, 0, 0)",
				DEFAULT_RATING,
			);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS applied_matches (
					match_id TEXT PRIMARY KEY
				)
			`);
		},
	}),
	actions: {
		initialize: async (c, input: { username: string }) => {
			// Only the first call sets the username.
			await c.db.execute(
				"UPDATE profile SET username = ? WHERE id = 1 AND username = ''",
				input.username,
			);
		},
		getProfile: async (c): Promise<PlayerSnapshot> => {
			const [profile] = await c.db.execute<PlayerSnapshot>(
				"SELECT username, rating, wins, losses FROM profile WHERE id = 1",
			);
			return profile!;
		},
		getRating: async (c): Promise<number> => {
			const [profile] = await c.db.execute<{ rating: number }>(
				"SELECT rating FROM profile WHERE id = 1",
			);
			return profile!.rating;
		},
		applyMatchResult: async (
			c,
			input: { matchId: string; won: boolean; newRating: number },
		) => {
			// Record the match id in the same transaction as the result, so a
			// report that is sent again is applied only once. The win or loss
			// is added to the saved row, so two different results at once are
			// both counted.
			const profile = await c.db.transaction(
				async (tx) => {
					const applied = await tx.execute(
						"INSERT INTO applied_matches (match_id) VALUES (?) ON CONFLICT DO NOTHING RETURNING match_id",
						input.matchId,
					);
					if (applied.length === 0) return null;

					const [updated] = await tx.execute<PlayerSnapshot>(
						"UPDATE profile SET rating = ?, wins = wins + ?, losses = losses + ? WHERE id = 1 RETURNING username, rating, wins, losses",
						input.newRating,
						input.won ? 1 : 0,
						input.won ? 0 : 1,
					);
					return updated!;
				},
				{ name: "apply-match-result" },
			);
			if (profile) c.broadcast("stateUpdate", profile);
		},
	},
});

export type PlayerSnapshot = {
	username: string;
	rating: number;
	wins: number;
	losses: number;
};
