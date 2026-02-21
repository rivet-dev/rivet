import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { hasInvalidInternalToken, isInternalToken } from "../../auth.ts";

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
	onBeforeConnect: (_c, params: { internalToken?: string }) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		if (invoke.kind === "action" && invoke.name === "updatePlayer") {
			return isInternal;
		}
		if (invoke.kind === "action" && invoke.name === "getTopScores") {
			return true;
		}
		if (invoke.kind === "subscribe" && invoke.name === "leaderboardUpdate") {
			return !isInternal;
		}
		return false;
	},
	actions: {
		updatePlayer: async (c, input: { username: string; rating: number; wins: number; losses: number }) => {
			await c.db.execute(
				`INSERT INTO leaderboard (username, rating, wins, losses, updated_at) VALUES (?, ?, ?, ?, ?)
				 ON CONFLICT(username) DO UPDATE SET rating = ?, wins = ?, losses = ?, updated_at = ?`,
				input.username,
				input.rating,
				input.wins,
				input.losses,
				Date.now(),
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
