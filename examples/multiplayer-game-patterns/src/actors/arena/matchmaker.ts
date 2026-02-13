import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { registry } from "../index.ts";
import { type Mode, MODE_CONFIG } from "./config.ts";

export interface ArenaAssignment {
	matchId: string;
	playerId: string;
	playerToken: string;
	teamId: number;
	mode: Mode;
}

export const arenaMatchmaker = actor({
	options: { name: "Arena - Matchmaker", icon: "crosshairs" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		queueForMatch: queue<{ mode: Mode }, { playerId: string }>(),
		matchCompleted: queue<{ matchId: string }>(),
	},
	actions: {
		registerPlayer: async (c, { playerId }: { playerId: string }) => {
			await c.db.execute(
				`UPDATE player_pool SET conn_id = ? WHERE player_id = ?`,
				c.conn.id,
				playerId,
			);
		},
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
				player_token: string;
				team_id: number;
				mode: string;
			}>(
				`SELECT * FROM assignments WHERE player_id = ?`,
				playerId,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				playerId: row.player_id,
				playerToken: row.player_token,
				teamId: row.team_id,
				mode: row.mode as Mode,
			};
		},
	},
	onDisconnect: async (c, conn) => {
		await c.db.execute(`DELETE FROM player_pool WHERE conn_id = ?`, conn.id);
		await broadcastQueueSizes(c);
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "queueForMatch") {
				const playerId = await processQueueEntry(c, message.body.mode);
				await message.complete({ playerId });
			} else if (message.name === "matchCompleted") {
				await c.db.execute(
					`DELETE FROM matches WHERE match_id = ?`,
					message.body.matchId,
				);
				await message.complete();
			}
		}
	},
});

async function broadcastQueueSizes(
	c: ActorContextOf<typeof arenaMatchmaker>,
) {
	const rows = await c.db.execute<{ mode: string; cnt: number }>(
		`SELECT mode, COUNT(*) as cnt FROM player_pool GROUP BY mode`,
	);
	const counts: Record<string, number> = {};
	for (const row of rows) {
		counts[row.mode] = row.cnt;
	}
	c.broadcast("queueUpdate", { counts });
}

async function processQueueEntry(
	c: ActorContextOf<typeof arenaMatchmaker>,
	mode: Mode,
): Promise<string> {
	const playerId = crypto.randomUUID();
	const config = MODE_CONFIG[mode];

	// Insert player into pool.
	await c.db.execute(
		`INSERT OR REPLACE INTO player_pool (player_id, mode, queued_at) VALUES (?, ?, ?)`,
		playerId,
		mode,
		Date.now(),
	);

	await broadcastQueueSizes(c);

	// Count queued players for this mode.
	const countRows = await c.db.execute<{ cnt: number }>(
		`SELECT COUNT(*) as cnt FROM player_pool WHERE mode = ?`,
		mode,
	);
	const count = countRows[0]?.cnt ?? 0;

	if (count >= config.capacity) {
		await fillMatch(c, mode, config);
	}

	return playerId;
}

async function fillMatch(
	c: ActorContextOf<typeof arenaMatchmaker>,
	mode: Mode,
	config: { capacity: number; teams: number },
) {
	// Pop oldest N players.
	const queued = await c.db.execute<{ player_id: string }>(
		`SELECT player_id FROM player_pool WHERE mode = ? ORDER BY queued_at ASC LIMIT ?`,
		mode,
		config.capacity,
	);

	const playerIds = queued.map((r) => r.player_id);

	// Remove from queue.
	for (const pid of playerIds) {
		await c.db.execute(`DELETE FROM player_pool WHERE player_id = ?`, pid);
	}

	// Assign teams and generate tokens.
	const matchId = crypto.randomUUID();
	const assignedPlayers = playerIds.map((pid, idx) => ({
		playerId: pid,
		token: crypto.randomUUID(),
		teamId: config.teams > 0 ? idx % config.teams : -1,
	}));

	// Create match actor with all players in input.
	const client = c.client<typeof registry>();
	await client.arenaMatch.create([matchId], {
		input: {
			matchId,
			mode,
			capacity: config.capacity,
			assignedPlayers: assignedPlayers.map((ap) => ({
				playerId: ap.playerId,
				token: ap.token,
				teamId: ap.teamId,
			})),
		},
	});

	// Insert match record.
	await c.db.execute(
		`INSERT INTO matches (match_id, mode, capacity, created_at) VALUES (?, ?, ?, ?)`,
		matchId,
		mode,
		config.capacity,
		Date.now(),
	);

	await broadcastQueueSizes(c);

	// Store assignments in DB so bots can poll for them.
	const assignments: ArenaAssignment[] = assignedPlayers.map((ap) => ({
		matchId,
		playerId: ap.playerId,
		playerToken: ap.token,
		teamId: ap.teamId,
		mode,
	}));
	for (const a of assignments) {
		await c.db.execute(
			`INSERT INTO assignments (player_id, match_id, player_token, team_id, mode) VALUES (?, ?, ?, ?, ?)`,
			a.playerId,
			a.matchId,
			a.playerToken,
			a.teamId,
			a.mode,
		);
	}

	// Broadcast assignments to WS connections.
	c.broadcast("assigned", { assignments });
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
			player_token TEXT NOT NULL,
			team_id INTEGER NOT NULL,
			mode TEXT NOT NULL
		)
	`);
}
