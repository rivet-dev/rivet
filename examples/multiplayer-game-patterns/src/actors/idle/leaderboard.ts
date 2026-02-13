import { actor, type ActorContextOf, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

export interface LeaderboardEntry {
	playerId: string;
	playerName: string;
	totalProduced: number;
}

export const idleLeaderboard = actor({
	options: { name: "Idle - Leaderboard", icon: "industry" },
	db: db({
		onMigrate: migrateTables,
	}),
	events: {
		leaderboardUpdate: event<LeaderboardEntry[]>(),
	},
	actions: {
		updateScore: async (
			c,
			input: { playerId: string; playerName: string; totalProduced: number },
		) => {
			await c.db.execute(
				`INSERT INTO scores (player_id, player_name, total_produced, updated_at) VALUES (?, ?, ?, ?)
				 ON CONFLICT(player_id) DO UPDATE SET player_name = ?, total_produced = ?, updated_at = ?`,
				input.playerId,
				input.playerName,
				input.totalProduced,
				Date.now(),
				input.playerName,
				input.totalProduced,
				Date.now(),
			);

			const top = await getTop(c, 10);
			c.broadcast("leaderboardUpdate", top);
		},
		getTopScores: async (c, input: { limit?: number }) => {
			return await getTop(c, input.limit ?? 10);
		},
	},
});

async function getTop(
	c: ActorContextOf<typeof idleLeaderboard>,
	limit: number,
): Promise<LeaderboardEntry[]> {
	const rows = await c.db.execute<{
		player_id: string;
		player_name: string;
		total_produced: number;
	}>(
		`SELECT player_id, player_name, total_produced FROM scores ORDER BY total_produced DESC LIMIT ?`,
		limit,
	);
	return rows.map((r) => ({
		playerId: r.player_id,
		playerName: r.player_name,
		totalProduced: r.total_produced,
	}));
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS scores (
			player_id TEXT PRIMARY KEY,
			player_name TEXT NOT NULL,
			total_produced INTEGER NOT NULL DEFAULT 0,
			updated_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(
		`CREATE INDEX IF NOT EXISTS scores_rank_idx ON scores (total_produced DESC)`,
	);
}
