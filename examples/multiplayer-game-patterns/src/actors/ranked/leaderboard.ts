import { type ActorContextOf, actor, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export interface LeaderboardEntry {
	username: string;
	rating: number;
	wins: number;
	losses: number;
}

export const rankedLeaderboard = actor({
	options: { name: "Ranked - Leaderboard", icon: "ranking-star" },
	db: db({
		onMigrate: migrateTables,
	}),
	events: {
		leaderboardUpdate: event<LeaderboardEntry[]>(),
	},
	actions: {
		updatePlayer: async (
			c,
			input: {
				username: string;
				rating: number;
				wins: number;
				losses: number;
			},
		) => {
			// Two matches for one player can finish at once and report out of
			// order. Every result adds one win or loss, so only a profile with
			// at least as many games as the saved row replaces it.
			await c.db.execute(
				`INSERT INTO leaderboard (username, rating, wins, losses, updated_at) VALUES (?, ?, ?, ?, ?)
					 ON CONFLICT(username) DO UPDATE SET rating = excluded.rating, wins = excluded.wins, losses = excluded.losses, updated_at = excluded.updated_at
					 WHERE excluded.wins + excluded.losses >= leaderboard.wins + leaderboard.losses`,
				input.username,
				input.rating,
				input.wins,
				input.losses,
				Date.now(),
			);
			const top = await getTop(c);
			c.broadcast("leaderboardUpdate", top);
		},
		getTopScores: async (c) => {
			return await getTop(c);
		},
	},
});

async function getTop(
	c: ActorContextOf<typeof rankedLeaderboard>,
): Promise<LeaderboardEntry[]> {
	const rows = await c.db.execute<{
		username: string;
		rating: number;
		wins: number;
		losses: number;
	}>(
		`SELECT username, rating, wins, losses FROM leaderboard ORDER BY rating DESC LIMIT 20`,
	);
	return rows.map((r) => ({
		username: r.username,
		rating: r.rating,
		wins: r.wins,
		losses: r.losses,
	}));
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS leaderboard (
			username TEXT PRIMARY KEY,
			rating INTEGER NOT NULL,
			wins INTEGER NOT NULL DEFAULT 0,
			losses INTEGER NOT NULL DEFAULT 0,
			updated_at INTEGER NOT NULL
		)
	`);
}
