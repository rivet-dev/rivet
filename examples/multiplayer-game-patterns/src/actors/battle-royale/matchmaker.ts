/*
This matchmaker uses a two phase join flow.
1. findMatch reserves a slot immediately and returns matchId, playerId, and reservationToken.
2. The client connects to the match actor with playerId and reservationToken.
3. The match actor confirms the reservation through confirmJoin before accepting the connection.
4. Pending reservations expire after JOIN_RESERVATION_TTL_MS, which marks never connected players as expired and releases slots.
5. updateMatch only updates connected player count and started state, so reserved slot counts are not clobbered.
*/
import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { registry } from "../index.ts";
import { LOBBY_CAPACITY } from "./config.ts";

const JOIN_RESERVATION_TTL_MS = 15_000;

type ReleaseReason = "disconnect_timeout" | "reservation_expired" | "invalid_join";

export const battleRoyaleMatchmaker = actor({
	options: { name: "Battle Royale - Matchmaker", icon: "skull-crossbones" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		findMatch: queue<
			Record<string, never>,
			{ matchId: string; playerId: string; reservationToken: string }
		>(),
		confirmJoin: queue<
			{ matchId: string; playerId: string; reservationToken: string },
			{ accepted: boolean }
		>(),
		releasePlayer: queue<{
			matchId: string;
			playerId: string;
			reservationToken: string;
			reason: ReleaseReason;
		}>(),
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
			await expirePendingReservations(c, now);

			if (message.name === "findMatch") {
				const result = await processFindMatch(c, now);
				await message.complete(result);
			} else if (message.name === "confirmJoin") {
				const result = await processConfirmJoin(c, message.body, now);
				await message.complete(result);
			} else if (message.name === "releasePlayer") {
				await processReleasePlayer(c, message.body, now);
				await message.complete();
			} else if (message.name === "updateMatch") {
				await c.db.execute(
					`UPDATE matches SET connected_player_count = ?, is_started = ?, updated_at = ? WHERE match_id = ?`,
					message.body.connectedPlayerCount,
					message.body.isStarted ? 1 : 0,
					now,
					message.body.matchId,
				);
				await message.complete();
			} else if (message.name === "closeMatch") {
				await c.db.execute(
					`DELETE FROM reservations WHERE match_id = ?`,
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
): Promise<{ matchId: string; playerId: string; reservationToken: string }> {
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
	const reservationToken = crypto.randomUUID();
	const reservationExpiresAt = now + JOIN_RESERVATION_TTL_MS;

	const client = c.client<typeof registry>();
	await client.battleRoyaleMatch
		.get([matchId])
		.createPlayer({ playerId, reservationToken, reservationExpiresAt });

	await c.db.execute(
		`INSERT INTO reservations (reservation_token, match_id, player_id, state, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?)`,
		reservationToken,
		matchId,
		playerId,
		"pending",
		reservationExpiresAt,
		now,
	);
	await syncReservedPlayerCount(c, matchId, now);

	return { matchId, playerId, reservationToken };
}

async function processConfirmJoin(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	input: { matchId: string; playerId: string; reservationToken: string },
	now: number,
): Promise<{ accepted: boolean }> {
	const rows = await c.db.execute<{ state: string; expires_at: number }>(
		`SELECT state, expires_at FROM reservations WHERE reservation_token = ? AND match_id = ? AND player_id = ? LIMIT 1`,
		input.reservationToken,
		input.matchId,
		input.playerId,
	);
	const row = rows[0];
	if (!row) {
		return { accepted: false };
	}
	if (row.state === "connected") {
		return { accepted: true };
	}
	if (row.state !== "pending") {
		return { accepted: false };
	}
	if (row.expires_at <= now) {
		await c.db.execute(
			`UPDATE reservations SET state = ?, closed_at = ?, close_reason = ? WHERE reservation_token = ? AND state = 'pending'`,
			"expired",
			now,
			"ttl_expired",
			input.reservationToken,
		);
		await syncReservedPlayerCount(c, input.matchId, now);
		return { accepted: false };
	}

	await c.db.execute(
		`UPDATE reservations SET state = ?, connected_at = ? WHERE reservation_token = ? AND state = 'pending'`,
		"connected",
		now,
		input.reservationToken,
	);
	await syncReservedPlayerCount(c, input.matchId, now);

	return { accepted: true };
}

async function processReleasePlayer(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	input: {
		matchId: string;
		playerId: string;
		reservationToken: string;
		reason: ReleaseReason;
	},
	now: number,
): Promise<void> {
	const rows = await c.db.execute<{ state: string }>(
		`SELECT state FROM reservations WHERE reservation_token = ? AND match_id = ? AND player_id = ? LIMIT 1`,
		input.reservationToken,
		input.matchId,
		input.playerId,
	);
	const row = rows[0];
	if (!row || row.state === "expired" || row.state === "released") {
		return;
	}

	const nextState = row.state === "pending" ? "expired" : "released";
	await c.db.execute(
		`UPDATE reservations SET state = ?, closed_at = ?, close_reason = ? WHERE reservation_token = ?`,
		nextState,
		now,
		input.reason,
		input.reservationToken,
	);
	await syncReservedPlayerCount(c, input.matchId, now);
}

async function expirePendingReservations(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	now: number,
): Promise<void> {
	const rows = await c.db.execute<{ match_id: string }>(
		`SELECT DISTINCT match_id FROM reservations WHERE state = 'pending' AND expires_at <= ?`,
		now,
	);
	if (rows.length === 0) {
		return;
	}

	await c.db.execute(
		`UPDATE reservations SET state = ?, closed_at = ?, close_reason = ? WHERE state = 'pending' AND expires_at <= ?`,
		"expired",
		now,
		"ttl_expired",
		now,
	);

	for (const row of rows) {
		await syncReservedPlayerCount(c, row.match_id, now);
	}
}

async function syncReservedPlayerCount(
	c: ActorContextOf<typeof battleRoyaleMatchmaker>,
	matchId: string,
	now: number,
): Promise<void> {
	const rows = await c.db.execute<{ cnt: number }>(
		`SELECT COUNT(*) as cnt FROM reservations WHERE match_id = ? AND state IN ('pending', 'connected')`,
		matchId,
	);
	await c.db.execute(
		`UPDATE matches SET player_count = ?, updated_at = ? WHERE match_id = ?`,
		rows[0]?.cnt ?? 0,
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
		CREATE TABLE IF NOT EXISTS reservations (
			reservation_token TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			player_id TEXT NOT NULL,
			state TEXT NOT NULL,
			expires_at INTEGER NOT NULL,
			created_at INTEGER NOT NULL,
			connected_at INTEGER,
			closed_at INTEGER,
			close_reason TEXT,
			UNIQUE(match_id, player_id)
		)
	`);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS matches_open_idx ON matches (is_started, player_count, created_at)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS reservations_match_state_idx ON reservations (match_id, state)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS reservations_expire_idx ON reservations (state, expires_at)",
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
