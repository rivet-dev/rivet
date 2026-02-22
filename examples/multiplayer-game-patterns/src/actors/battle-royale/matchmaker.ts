/*
This matchmaker uses a two phase join flow.
1. findMatch claims a slot immediately and returns matchId and playerId.
2. The client connects to the match actor with playerId.
3. The match actor claims that pending player through pendingPlayerConnected before first join.
4. Pending players expire after JOIN_RESERVATION_TTL_MS, which removes never connected players.
5. updateMatch reports occupied player count and started state, while player_count stays pending + occupied.
*/
import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { registry } from "../index.ts";
import { LOBBY_CAPACITY } from "./config.ts";

const JOIN_RESERVATION_TTL_MS = 15_000;

export const battleRoyaleMatchmaker = actor({
	options: { name: "Battle Royale - Matchmaker", icon: "skull-crossbones" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		findMatch: queue<
			Record<string, never>,
			{ matchId: string; playerId: string }
		>(),
		pendingPlayerConnected: queue<
			{ matchId: string; playerId: string },
			{ accepted: boolean }
		>(),
		updateMatch: queue<{
			matchId: string;
			connectedPlayerCount: number;
			isStarted: boolean;
		}>(),
		closeMatch: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			const now = Date.now();
			await expirePendingPlayers(c, now);

			if (message.name === "findMatch") {
				const result = await processFindMatch(c, now);
				await message.complete(result);
			} else if (message.name === "pendingPlayerConnected") {
				const result = await processPendingPlayerConnected(c, message.body, now);
				await message.complete(result);
			} else if (message.name === "updateMatch") {
				await c.db.execute(
					`UPDATE matches SET connected_player_count = ?, is_started = ?, updated_at = ? WHERE match_id = ?`,
					message.body.connectedPlayerCount,
					message.body.isStarted ? 1 : 0,
					now,
					message.body.matchId,
				);
				await syncClaimedPlayerCount(c, message.body.matchId, now);
				await message.complete();
			} else if (message.name === "closeMatch") {
				await c.db.execute(
					`DELETE FROM pending_players WHERE match_id = ?`,
					message.body.matchId,
				);
				await c.db.execute(
					`DELETE FROM matches WHERE match_id = ?`,
					message.body.matchId,
				);
				await message.complete();
			}
		}
	},
});

async function processFindMatch(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	now: number,
): Promise<{ matchId: string; playerId: string }> {
	const rows = await c.db.execute<{ match_id: string }>(
		`SELECT match_id FROM matches WHERE is_started = 0 AND player_count < ? ORDER BY player_count DESC, created_at ASC LIMIT 1`,
		LOBBY_CAPACITY,
	);
	let matchId = rows[0]?.match_id ?? null;

	if (!matchId) {
		matchId = crypto.randomUUID();
		await c.db.execute(
			`INSERT INTO matches (match_id, player_count, connected_player_count, is_started, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)`,
			matchId,
			0,
			0,
			0,
			now,
			now,
		);
		const client = c.client<typeof registry>();
		await client.battleRoyaleMatch.create([matchId], {
			input: { matchId },
		});
	}

	const playerId = crypto.randomUUID();
	const expiresAt = now + JOIN_RESERVATION_TTL_MS;

	await c.db.execute(
		`INSERT INTO pending_players (match_id, player_id, expires_at, created_at) VALUES (?, ?, ?, ?)`,
		matchId,
		playerId,
		expiresAt,
		now,
	);
	await syncClaimedPlayerCount(c, matchId, now);

	return { matchId, playerId };
}

async function processPendingPlayerConnected(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	input: { matchId: string; playerId: string },
	now: number,
): Promise<{ accepted: boolean }> {
	const rows = await c.db.execute<{ expires_at: number }>(
		`SELECT expires_at FROM pending_players WHERE match_id = ? AND player_id = ? LIMIT 1`,
		input.matchId,
		input.playerId,
	);
	const row = rows[0];
	if (!row) {
		return { accepted: false };
	}
	if (row.expires_at <= now) {
		await c.db.execute(
			`DELETE FROM pending_players WHERE match_id = ? AND player_id = ?`,
			input.matchId,
			input.playerId,
		);
		await syncClaimedPlayerCount(c, input.matchId, now);
		return { accepted: false };
	}

	await c.db.execute(
		`DELETE FROM pending_players WHERE match_id = ? AND player_id = ?`,
		input.matchId,
		input.playerId,
	);
	await syncClaimedPlayerCount(c, input.matchId, now);

	return { accepted: true };
}

async function expirePendingPlayers(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	now: number,
): Promise<void> {
	const rows = await c.db.execute<{ match_id: string }>(
		`SELECT DISTINCT match_id FROM pending_players WHERE expires_at <= ?`,
		now,
	);
	if (rows.length === 0) {
		return;
	}

	await c.db.execute(
		`DELETE FROM pending_players WHERE expires_at <= ?`,
		now,
	);

	for (const row of rows) {
		await syncClaimedPlayerCount(c, row.match_id, now);
	}
}

async function syncClaimedPlayerCount(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	matchId: string,
	now: number,
): Promise<void> {
	await c.db.execute(
		`UPDATE matches
			SET player_count = connected_player_count + COALESCE(
				(SELECT COUNT(*) FROM pending_players WHERE match_id = ?),
				0
			),
			updated_at = ?
		WHERE match_id = ?`,
		matchId,
		now,
		matchId,
	);
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			player_count INTEGER NOT NULL,
			connected_player_count INTEGER NOT NULL DEFAULT 0,
			is_started INTEGER NOT NULL DEFAULT 0,
			created_at INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	await ensureColumn(
		dbHandle,
		"matches",
		"connected_player_count",
		"INTEGER NOT NULL DEFAULT 0",
	);
	await ensureColumn(
		dbHandle,
		"matches",
		"updated_at",
		"INTEGER NOT NULL DEFAULT 0",
	);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS pending_players (
			match_id TEXT NOT NULL,
			player_id TEXT NOT NULL,
			expires_at INTEGER NOT NULL,
			created_at INTEGER NOT NULL,
			PRIMARY KEY (match_id, player_id)
		)
	`);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS matches_open_idx ON matches (is_started, player_count, created_at)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS pending_players_match_idx ON pending_players (match_id)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS pending_players_expire_idx ON pending_players (expires_at)",
	);
}

async function ensureColumn(
	dbHandle: RawAccess,
	table: "matches",
	column: "connected_player_count" | "updated_at",
	definition: "INTEGER NOT NULL DEFAULT 0",
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
