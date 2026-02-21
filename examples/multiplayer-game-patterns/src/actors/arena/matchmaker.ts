import { actor, type ActorContextOf, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
import { registry } from "../index.ts";
import { type Mode, MODE_CONFIG } from "./config.ts";

export interface ArenaAssignment {
	matchId: string;
	playerId: string;
	playerToken: string;
	teamId: number;
	mode: Mode;
	connId: string | null;
}

type QueuePlayerRow = {
	player_id: string;
	conn_id: string | null;
	registration_token: string | null;
};

type StoredArenaAssignment = ArenaAssignment & {
	registrationToken: string | null;
};

export const arenaMatchmaker = actor({
	options: { name: "Arena - Matchmaker", icon: "crosshairs" },
	db: db({
		onMigrate: migrateTables,
	}),
	onBeforeConnect: (_c, params: { internalToken?: string }) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		if (invoke.kind === "queue" && invoke.name === "queueForMatch") {
			return !isInternal;
		}
		if (invoke.kind === "queue" && invoke.name === "matchCompleted") {
			return isInternal;
		}
		if (
			invoke.kind === "action" &&
			(invoke.name === "registerPlayer" ||
				invoke.name === "getQueueSizes" ||
				invoke.name === "getAssignment")
		) {
			return !isInternal;
		}
		if (invoke.kind === "subscribe" && invoke.name === "queueUpdate") {
			return !isInternal;
		}
		return false;
	},
	queues: {
		queueForMatch: queue<
			{ mode: Mode },
			{ playerId: string; registrationToken: string }
		>(),
		matchCompleted: queue<{ matchId: string }>(),
	},
	actions: {
		registerPlayer: async (
			c,
			{
				playerId,
				registrationToken,
			}: { playerId: string; registrationToken: string },
		) => {
			await c.db.execute(
				`UPDATE player_pool SET conn_id = ? WHERE player_id = ? AND registration_token = ?`,
				c.conn.id,
				playerId,
				registrationToken,
			);
			const playerPoolChangeRows = await c.db.execute<{ changed: number }>(
				`SELECT changes() AS changed`,
			);
			const playerPoolChanges = playerPoolChangeRows[0]?.changed ?? 0;
			await c.db.execute(
				`UPDATE assignments SET conn_id = ? WHERE player_id = ? AND registration_token = ?`,
				c.conn.id,
				playerId,
				registrationToken,
			);
			const assignmentChangeRows = await c.db.execute<{ changed: number }>(
				`SELECT changes() AS changed`,
			);
			const assignmentChanges = assignmentChangeRows[0]?.changed ?? 0;
			if (playerPoolChanges === 0 && assignmentChanges === 0) {
				throw new UserError("forbidden", { code: "forbidden" });
			}
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
		getAssignment: async (
			c,
			{
				playerId,
				registrationToken,
			}: { playerId: string; registrationToken: string },
		) => {
			const rows = await c.db.execute<{
				match_id: string;
				player_id: string;
				player_token: string;
				team_id: number;
				mode: string;
				conn_id: string | null;
			}>(
				`SELECT * FROM assignments WHERE player_id = ? AND conn_id = ? AND registration_token = ?`,
				playerId,
				c.conn.id,
				registrationToken,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				playerId: row.player_id,
				playerToken: row.player_token,
				teamId: row.team_id,
				mode: row.mode as Mode,
				connId: row.conn_id,
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
				const queueResult = await processQueueEntry(c, message.body.mode);
				await message.complete(queueResult);
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
): Promise<{ playerId: string; registrationToken: string }> {
	const playerId = crypto.randomUUID();
	const registrationToken = crypto.randomUUID();
	const config = MODE_CONFIG[mode];

	// Insert player into pool.
	await c.db.execute(
		`INSERT OR REPLACE INTO player_pool (player_id, mode, queued_at, registration_token) VALUES (?, ?, ?, ?)`,
		playerId,
		mode,
		Date.now(),
		registrationToken,
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

	return { playerId, registrationToken };
}

async function fillMatch(
	c: ActorContextOf<typeof arenaMatchmaker>,
	mode: Mode,
	config: { capacity: number; teams: number },
) {
	// Pop oldest N players.
	const queued = await c.db.execute<QueuePlayerRow>(
		`SELECT player_id, conn_id, registration_token FROM player_pool WHERE mode = ? ORDER BY queued_at ASC LIMIT ?`,
		mode,
		config.capacity,
	);

	const queuedPlayers = queued.map((r) => ({
		playerId: r.player_id,
		connId: r.conn_id,
		registrationToken: r.registration_token,
	}));
	const playerIds = queuedPlayers.map((r) => r.playerId);

	// Remove from queue.
	for (const pid of playerIds) {
		await c.db.execute(`DELETE FROM player_pool WHERE player_id = ?`, pid);
	}

	// Assign teams and generate tokens.
	const matchId = crypto.randomUUID();
	const assignedPlayers = queuedPlayers.map((queuedPlayer, idx) => ({
		playerId: queuedPlayer.playerId,
		token: crypto.randomUUID(),
		connId: queuedPlayer.connId,
		registrationToken: queuedPlayer.registrationToken,
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
	const assignments: StoredArenaAssignment[] = assignedPlayers.map((ap) => ({
		matchId,
		playerId: ap.playerId,
		playerToken: ap.token,
		teamId: ap.teamId,
		mode,
		connId: ap.connId,
		registrationToken: ap.registrationToken,
	}));
	for (const a of assignments) {
		await c.db.execute(
			`INSERT INTO assignments (player_id, match_id, player_token, team_id, mode, conn_id, registration_token) VALUES (?, ?, ?, ?, ?, ?, ?)`,
			a.playerId,
			a.matchId,
			a.playerToken,
			a.teamId,
			a.mode,
			a.connId,
			a.registrationToken,
		);
	}
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_pool (
			player_id TEXT PRIMARY KEY,
			mode TEXT NOT NULL,
			queued_at INTEGER NOT NULL,
			conn_id TEXT,
			registration_token TEXT
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
			mode TEXT NOT NULL,
			conn_id TEXT,
			registration_token TEXT
		)
	`);
	await ensureColumn(dbHandle, "player_pool", "registration_token", "TEXT");
	await ensureColumn(dbHandle, "assignments", "registration_token", "TEXT");
}

async function ensureColumn(
	dbHandle: RawAccess,
	table: "player_pool" | "assignments",
	column: "registration_token",
	definition: "TEXT",
) {
	const columns = await dbHandle.execute<{ name: string }>(
		`PRAGMA table_info(${table})`,
	);
	if (!columns.some((col) => col.name === column)) {
		await dbHandle.execute(
			`ALTER TABLE ${table} ADD COLUMN ${column} ${definition}`,
		);
	}
}
