import { actor, type ActorContextOf, event, queue } from "rivetkit";
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
	queues: {
		updatePlayer: queue<{
			username: string;
			rating: number;
			wins: number;
			losses: number;
		}>(),
	},
	events: {
		leaderboardUpdate: event<LeaderboardEntry[]>(),
	},
	actions: {
		updatePlayer: async (c, input: { username: string; rating: number; wins: number; losses: number }) => {
			await c.queue.send("updatePlayer", input);
		},
		getTopScores: async (c) => {
			return await getTop(c);
		},
	},
	run: async (c) => {
		for await (const message of c.queue.iter()) {
			if (message.name !== "updatePlayer") continue;

			await c.db.execute(
				`INSERT INTO leaderboard (username, rating, wins, losses, updated_at) VALUES (?, ?, ?, ?, ?)
					 ON CONFLICT(username) DO UPDATE SET rating = ?, wins = ?, losses = ?, updated_at = ?`,
				message.body.username,
				message.body.rating,
				message.body.wins,
				message.body.losses,
				Date.now(),
				message.body.rating,
				message.body.wins,
				message.body.losses,
				Date.now(),
			);
			const top = await getTop(c);
			c.broadcast("leaderboardUpdate", top);
		}
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
	}>(`SELECT username, rating, wins, losses FROM leaderboard ORDER BY rating DESC LIMIT 20`);
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
