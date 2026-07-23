/*
This matchmaker uses a two phase open lobby flow.
1. findLobby claims a slot immediately and returns matchId and playerId.
2. The client connects to the match actor with playerId.
3. The match actor claims that pending player through pendingPlayerConnected before first join.
4. Pending players expire after JOIN_RESERVATION_TTL_MS, which removes never connected players.
5. updateMatch reports occupied player count while player_count stays pending + occupied.
*/
import { type ActorContextOf, actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import type { registry } from "../index.ts";
import { CAPACITY } from "./config.ts";

const JOIN_RESERVATION_TTL_MS = 15_000;

export const ioStyleMatchmaker = actor({
	options: { name: "IO - Matchmaker", icon: "earth-americas" },
	db: db({
		onMigrate: migrateTables,
	}),
	actions: {
		findLobby: async (c) => {
			const now = Date.now();
			await expirePendingPlayers(c, now);
			return processFindLobby(c, now);
		},
		pendingPlayerConnected: async (
			c,
			input: { matchId: string; playerId: string },
		) => {
			const now = Date.now();
			await expirePendingPlayers(c, now);
			return processPendingPlayerConnected(c, input, now);
		},
		updateMatch: async (
			c,
			input: { matchId: string; connectedPlayerCount: number },
		) => {
			const now = Date.now();
			await c.db.execute(
				`UPDATE matches SET connected_player_count = ?, updated_at = ? WHERE match_id = ?`,
				input.connectedPlayerCount,
				now,
				input.matchId,
			);
			await syncClaimedPlayerCount(c, input.matchId, now);
		},
		closeMatch: async (c, input: { matchId: string }) => {
			await c.db.execute(
				`DELETE FROM pending_players WHERE match_id = ?`,
				input.matchId,
			);
			await c.db.execute(
				`DELETE FROM matches WHERE match_id = ?`,
				input.matchId,
			);
		},
	},
});

async function processFindLobby(
	c: ActorContextOf<typeof ioStyleMatchmaker>,
	now: number,
): Promise<{ matchId: string; playerId: string }> {
	const rows = await c.db.execute<{ match_id: string; player_count: number }>(
		`SELECT match_id, player_count FROM matches WHERE player_count < ? ORDER BY player_count DESC, updated_at DESC LIMIT 1`,
		CAPACITY,
	);
	let matchId = rows[0]?.match_id ?? null;

	if (!matchId) {
		matchId = crypto.randomUUID();
		await c.db.execute(
			`INSERT INTO matches (match_id, player_count, connected_player_count, updated_at) VALUES (?, ?, ?, ?)`,
			matchId,
			0,
			0,
			now,
		);
		const client = c.client<typeof registry>();
		await client.ioStyleMatch.create([matchId], {
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
	c: ActorContextOf<typeof ioStyleMatchmaker>,
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
	c: ActorContextOf<typeof ioStyleMatchmaker>,
	now: number,
): Promise<void> {
	const rows = await c.db.execute<{ match_id: string }>(
		`SELECT DISTINCT match_id FROM pending_players WHERE expires_at <= ?`,
		now,
	);
	if (rows.length === 0) return;

	await c.db.execute(
		`DELETE FROM pending_players WHERE expires_at <= ?`,
		now,
	);

	for (const row of rows) {
		await syncClaimedPlayerCount(c, row.match_id, now);
	}
}

async function syncClaimedPlayerCount(
	c: ActorContextOf<typeof ioStyleMatchmaker>,
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
		"CREATE INDEX IF NOT EXISTS matches_open_idx ON matches (player_count, updated_at)",
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
