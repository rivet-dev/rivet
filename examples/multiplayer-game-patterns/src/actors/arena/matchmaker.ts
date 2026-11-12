/*
This matchmaker uses a queue and fill flow.
1. queueForMatch adds players to player_pool for a specific mode.
2. Once enough players are queued for that mode, fillMatch removes them from the pool and creates a match actor.
3. The matchmaker stores assignments and broadcasts assignmentReady so each client can connect to the new match.
4. onDisconnect unqueues pending players so stale queue entries do not stay in the pool.
*/
import { type ActorContextOf, actor, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import type { registry } from "../index.ts";
import { MODE_CONFIG, type Mode } from "./config.ts";

export interface ArenaAssignment {
	matchId: string;
	playerId: string;
	teamId: number;
	mode: Mode;
	connId: string | null;
}

type QueuePlayerRow = {
	player_id: string;
	conn_id: string | null;
	queued_at: number;
};

export const arenaMatchmaker = actor({
	options: { name: "Arena - Matchmaker", icon: "crosshairs" },
	db: db({
		onMigrate: migrateTables,
	}),
	events: {
		assignmentReady: event<ArenaAssignment>(),
		queueUpdate: event<{ counts: Record<string, number> }>(),
	},
	actions: {
		// docs:start queue
		queueForMatch: async (c, { mode }: { mode: Mode }) => {
			const playerId = crypto.randomUUID();
			await c.db.execute(
				`INSERT INTO player_pool (player_id, mode, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
				playerId,
				mode,
				Date.now(),
				c.conn.id,
			);
			await broadcastQueueSizes(c);
			await fillMatch(c, mode);
			return { playerId };
		},
		// docs:end queue
		getQueueSizes: async (c) => {
			const rows = await c.db.execute<{ mode: string; cnt: number }>(
				`SELECT mode, COUNT(*) as cnt FROM player_pool GROUP BY mode`,
			);
			const counts: Record<string, number> = {};
			for (const row of rows) {
				counts[row.mode] = row.cnt;
			}
			return counts;
		},
		getAssignment: async (c, { playerId }: { playerId: string }) => {
			const rows = await c.db.execute<{
				match_id: string;
				player_id: string;
				team_id: number;
				mode: string;
				conn_id: string | null;
			}>(
				`SELECT * FROM assignments WHERE player_id = ? AND conn_id = ?`,
				playerId,
				c.conn.id,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				playerId: row.player_id,
				teamId: row.team_id,
				mode: row.mode as Mode,
				connId: row.conn_id,
			};
		},
		matchCompleted: async (c, { matchId }: { matchId: string }) => {
			await c.db.execute(`DELETE FROM matches WHERE match_id = ?`, matchId);
		},
	},
	// docs:start queue
	onDisconnect: async (c, conn) => {
		await c.db.execute(`DELETE FROM player_pool WHERE conn_id = ?`, conn.id);
		await broadcastQueueSizes(c);
	},
	// docs:end queue
});

async function broadcastQueueSizes(c: ActorContextOf<typeof arenaMatchmaker>) {
	const rows = await c.db.execute<{ mode: string; cnt: number }>(
		`SELECT mode, COUNT(*) as cnt FROM player_pool GROUP BY mode`,
	);
	const counts: Record<string, number> = {};
	for (const row of rows) {
		counts[row.mode] = row.cnt;
	}
	c.broadcast("queueUpdate", { counts });
}

// docs:start fill-match
async function fillMatch(c: ActorContextOf<typeof arenaMatchmaker>, mode: Mode) {
	const config = MODE_CONFIG[mode];

	// Actions run concurrently, so check the count and remove the players in
	// one transaction. Two players joining at once can't both fill the last slot.
	const queued = await c.db.transaction(
		async (tx) => {
			const rows = await tx.execute<QueuePlayerRow>(
				`SELECT player_id, conn_id, queued_at FROM player_pool WHERE mode = ? ORDER BY queued_at ASC LIMIT ?`,
				mode,
				config.capacity,
			);
			if (rows.length < config.capacity) return [];
			for (const row of rows) {
				await tx.execute(
					`DELETE FROM player_pool WHERE player_id = ?`,
					row.player_id,
				);
			}
			return rows;
		},
		{ name: "claim-players" },
	);
	if (queued.length === 0) return;

	const matchId = crypto.randomUUID();
	const assignedPlayers = queued.map((row, idx) => ({
		playerId: row.player_id,
		connId: row.conn_id,
		teamId: config.teams > 0 ? idx % config.teams : -1,
	}));

	// Create the match outside the transaction, because a transaction must not
	// wait on another actor.
	const client = c.client<typeof registry>();
	try {
		await client.arenaMatch.create([matchId], {
			input: {
				matchId,
				mode,
				capacity: config.capacity,
				assignedPlayers: assignedPlayers.map((ap) => ({
					playerId: ap.playerId,
					teamId: ap.teamId,
				})),
			},
		});
	} catch (error) {
		await returnToPool(c, mode, queued);
		throw error;
	}

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				`INSERT INTO matches (match_id, mode, capacity, created_at) VALUES (?, ?, ?, ?)`,
				matchId,
				mode,
				config.capacity,
				Date.now(),
			);
			for (const ap of assignedPlayers) {
				await tx.execute(
					`INSERT INTO assignments (player_id, match_id, team_id, mode, conn_id) VALUES (?, ?, ?, ?, ?)`,
					ap.playerId,
					matchId,
					ap.teamId,
					mode,
					ap.connId,
				);
			}
		},
		{ name: "save-match" },
	);

	await broadcastQueueSizes(c);

	for (const ap of assignedPlayers) {
		c.broadcast("assignmentReady", {
			matchId,
			playerId: ap.playerId,
			teamId: ap.teamId,
			mode,
			connId: ap.connId,
		} satisfies ArenaAssignment);
	}
}
// docs:end fill-match

/** Puts claimed players back at their original place in the pool. */
async function returnToPool(
	c: ActorContextOf<typeof arenaMatchmaker>,
	mode: Mode,
	players: QueuePlayerRow[],
) {
	// A player who left while the match was being created stays out of the
	// pool, because their disconnect found no row to remove.
	const stillConnected = players.filter(
		(player) => player.conn_id !== null && c.conns.has(player.conn_id),
	);

	await c.db.transaction(
		async (tx) => {
			for (const player of stillConnected) {
				await tx.execute(
					`INSERT INTO player_pool (player_id, mode, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
					player.player_id,
					mode,
					player.queued_at,
					player.conn_id,
				);
			}
		},
		{ name: "return-players" },
	);
	await broadcastQueueSizes(c);
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_pool (
			player_id TEXT PRIMARY KEY,
			mode TEXT NOT NULL,
			queued_at INTEGER NOT NULL,
			conn_id TEXT
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			mode TEXT NOT NULL,
			capacity INTEGER NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			team_id INTEGER NOT NULL,
			mode TEXT NOT NULL,
			conn_id TEXT
		)
	`);
}
