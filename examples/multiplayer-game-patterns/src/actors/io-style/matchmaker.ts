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

interface MatchmakerVars {
	// The lobby being created right now, so requests that find every lobby
	// full at the same time wait for it instead of opening one each
	openingLobby: Promise<void> | null;
}

export const ioStyleMatchmaker = actor({
	options: { name: "IO - Matchmaker", icon: "earth-americas" },
	db: db({
		onMigrate: migrateTables,
	}),
	createVars: (): MatchmakerVars => ({ openingLobby: null }),
	actions: {
		// docs:start actions
		findLobby: async (
			c,
		): Promise<{ matchId: string; playerId: string }> => {
			const playerId = crypto.randomUUID();
			let matchId = await reserveSeat(c, playerId);
			while (matchId === null) {
				await openLobby(c);
				matchId = await reserveSeat(c, playerId);
			}
			return { matchId, playerId };
		},
		pendingPlayerConnected: async (
			c,
			input: { matchId: string; playerId: string },
		): Promise<{ accepted: boolean }> => {
			const now = Date.now();
			return await c.db.transaction(
				async (tx) => {
					// Deleting the reservation is what claims it, so a player
					// who connects twice at once is accepted only once.
					const rows = await tx.execute<{ expires_at: number }>(
						`DELETE FROM pending_players WHERE match_id = ? AND player_id = ? RETURNING expires_at`,
						input.matchId,
						input.playerId,
					);
					const reservation = rows[0];
					if (!reservation) {
						return { accepted: false };
					}

					const accepted = reservation.expires_at > now;
					if (accepted) {
						// Count the seat as occupied right away. Otherwise it would
						// look free until the match reports its new count.
						await tx.execute(
							`UPDATE matches SET connected_player_count = connected_player_count + 1 WHERE match_id = ?`,
							input.matchId,
						);
					}
					await syncClaimedPlayerCount(tx, input.matchId, now);
					return { accepted };
				},
				{ name: "claim-seat" },
			);
		},
		updateMatch: async (
			c,
			input: { matchId: string; connectedPlayerCount: number },
		) => {
			const now = Date.now();
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						`UPDATE matches SET connected_player_count = ?, updated_at = ? WHERE match_id = ?`,
						input.connectedPlayerCount,
						now,
						input.matchId,
					);
					await syncClaimedPlayerCount(tx, input.matchId, now);
				},
				{ name: "update-match" },
			);
		},
		closeMatch: async (c, input: { matchId: string }) => {
			// Delete both in one transaction, so findLobby can't reserve a seat
			// in a match that is halfway closed.
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						`DELETE FROM pending_players WHERE match_id = ?`,
						input.matchId,
					);
					await tx.execute(
						`DELETE FROM matches WHERE match_id = ?`,
						input.matchId,
					);
				},
				{ name: "close-match" },
			);
		},
		// docs:end actions
	},
});

// docs:start find-lobby
/**
 * Reserves a seat for `playerId` in the fullest lobby with room and returns
 * its match id, or null when every lobby is full.
 */
async function reserveSeat(
	c: ActorContextOf<typeof ioStyleMatchmaker>,
	playerId: string,
): Promise<string | null> {
	const now = Date.now();

	// Actions run concurrently, so pick the lobby and take the seat in one
	// transaction. Two players asking at once can't both take the last seat.
	return await c.db.transaction(
		async (tx) => {
			await expirePendingPlayers(tx, now);

			const rows = await tx.execute<{ match_id: string }>(
				`SELECT match_id FROM matches WHERE player_count < ? ORDER BY player_count DESC, updated_at DESC LIMIT 1`,
				CAPACITY,
			);
			const matchId = rows[0]?.match_id;
			if (!matchId) {
				return null;
			}

			await tx.execute(
				`INSERT INTO pending_players (match_id, player_id, expires_at, created_at) VALUES (?, ?, ?, ?)`,
				matchId,
				playerId,
				now + JOIN_RESERVATION_TTL_MS,
				now,
			);
			await syncClaimedPlayerCount(tx, matchId, now);
			return matchId;
		},
		{ name: "reserve-seat" },
	);
}

/**
 * Opens one new empty lobby. Requests that find every lobby full at the same
 * time share the lobby that is already being opened.
 */
function openLobby(c: ActorContextOf<typeof ioStyleMatchmaker>): Promise<void> {
	if (!c.vars.openingLobby) {
		c.vars.openingLobby = createLobby(c).finally(() => {
			c.vars.openingLobby = null;
		});
	}
	return c.vars.openingLobby;
}

async function createLobby(
	c: ActorContextOf<typeof ioStyleMatchmaker>,
): Promise<void> {
	const matchId = crypto.randomUUID();

	// Create the match before saving its row, so findLobby never hands out a
	// match that doesn't exist yet. This can't run inside a transaction,
	// because a transaction must not wait on another actor.
	const client = c.client<typeof registry>();
	await client.ioStyleMatch.create([matchId]);

	await c.db.execute(
		`INSERT INTO matches (match_id, player_count, connected_player_count, updated_at) VALUES (?, ?, ?, ?)`,
		matchId,
		0,
		0,
		Date.now(),
	);
}
// docs:end find-lobby

async function expirePendingPlayers(tx: RawAccess, now: number): Promise<void> {
	const rows = await tx.execute<{ match_id: string }>(
		`SELECT DISTINCT match_id FROM pending_players WHERE expires_at <= ?`,
		now,
	);
	if (rows.length === 0) return;

	await tx.execute(`DELETE FROM pending_players WHERE expires_at <= ?`, now);

	for (const row of rows) {
		await syncClaimedPlayerCount(tx, row.match_id, now);
	}
}

async function syncClaimedPlayerCount(
	tx: RawAccess,
	matchId: string,
	now: number,
): Promise<void> {
	await tx.execute(
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
