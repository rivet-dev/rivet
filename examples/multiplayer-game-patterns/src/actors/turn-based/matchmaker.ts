/*
This matchmaker supports invite and queued public matchmaking flows.
1. createGame creates a private match with the first player as X.
2. joinByCode joins an existing private match by invite code as O.
3. queueForMatch action adds players to the public pool.
4. When two players are queued, the matchmaker creates a match and stores public assignments without invite codes.
5. getAssignment lets clients poll for their match assignment.
6. onDisconnect unqueues waiting players so stale queue entries do not remain.
*/
import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import type { registry } from "../index.ts";
import { generateInviteCode } from "./config.ts";

export interface TurnBasedAssignment {
	matchId: string;
	playerId: string;
	inviteCode?: string;
	connId: string | null;
}

type QueuePlayerRow = {
	player_id: string;
	player_name: string;
	queued_at: number;
	conn_id: string | null;
};

export const turnBasedMatchmaker = actor({
	options: { name: "Turn-Based - Matchmaker", icon: "chess-board" },
	db: db({
		onMigrate: migrateTables,
	}),
	events: {
		assignmentReady: event<TurnBasedAssignment>(),
	},
	actions: {
		// docs:start invite
		createGame: async (c, { playerName }: { playerName: string }) => {
			const matchId = crypto.randomUUID();
			const inviteCode = generateInviteCode();
			const playerId = crypto.randomUUID();

			const client = c.client<typeof registry>();
			await client.turnBasedMatch.create([matchId]);

			await client.turnBasedMatch.get([matchId]).createPlayer({
				playerId,
				playerName,
				symbol: "X" as const,
			});

			await c.db.execute(
				`INSERT INTO matches (match_id, invite_code, player_count, is_open_pool, created_at) VALUES (?, ?, ?, ?, ?)`,
				matchId,
				inviteCode,
				1,
				0,
				Date.now(),
			);

			return { matchId, playerId, inviteCode };
		},
		joinByCode: async (
			c,
			input: { inviteCode: string; playerName: string },
		) => {
			const code = input.inviteCode.toUpperCase().trim();

			// Actions run concurrently, so check and take the open seat in one
			// transaction. Two players joining with the same code can't both
			// get it.
			const matchId = await c.db.transaction(
				async (tx) => {
					const [match] = await tx.execute<{
						match_id: string;
						player_count: number;
					}>(
						`SELECT match_id, player_count FROM matches WHERE invite_code = ?`,
						code,
					);
					if (!match)
						throw new UserError("Game not found", {
							code: "game_not_found",
						});
					if (match.player_count >= 2)
						throw new UserError("Game is full", {
							code: "game_full",
						});

					await tx.execute(
						`UPDATE matches SET player_count = 2 WHERE match_id = ?`,
						match.match_id,
					);
					return match.match_id;
				},
				{ name: "claim-seat" },
			);

			// Seat the player outside the transaction, because a transaction
			// must not wait on another actor.
			const playerId = crypto.randomUUID();
			const client = c.client<typeof registry>();
			try {
				await client.turnBasedMatch.get([matchId]).createPlayer({
					playerId,
					playerName: input.playerName,
					symbol: "O" as const,
				});
			} catch (error) {
				// Open the seat again, so the code still works for someone else.
				await c.db.execute(
					`UPDATE matches SET player_count = 1 WHERE match_id = ?`,
					matchId,
				);
				throw error;
			}

			return { matchId, playerId };
		},
		// docs:end invite
		// docs:start queue
		queueForMatch: async (c, { playerName }: { playerName: string }) => {
			const playerId = crypto.randomUUID();
			await c.db.execute(
				`INSERT OR REPLACE INTO player_pool (player_id, player_name, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
				playerId,
				playerName,
				Date.now(),
				c.conn.id,
			);
			await attemptPairing(c);
			return { playerId };
		},
		// docs:end queue
		getAssignment: async (c, { playerId }: { playerId: string }) => {
			const rows = await c.db.execute<{
				match_id: string;
				player_id: string;
				invite_code: string | null;
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
				inviteCode: row.invite_code ?? undefined,
				connId: row.conn_id,
			} satisfies TurnBasedAssignment;
		},
		closeMatch: async (c, { matchId }: { matchId: string }) => {
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						`DELETE FROM assignments WHERE match_id = ?`,
						matchId,
					);
					await tx.execute(
						`DELETE FROM matches WHERE match_id = ?`,
						matchId,
					);
				},
				{ name: "close-match" },
			);
		},
	},
	// docs:start queue
	onDisconnect: async (c, conn) => {
		await c.db.execute(
			`DELETE FROM player_pool WHERE conn_id = ?`,
			conn.id,
		);
	},
	// docs:end queue
});

// docs:start queue
async function attemptPairing(
	c: ActorContextOf<typeof turnBasedMatchmaker>,
): Promise<void> {
	// Actions run concurrently, so take the two oldest players in one
	// transaction. Two players queueing at once can't both pair with the
	// same opponent.
	const queued = await c.db.transaction(
		async (tx) => {
			const rows = await tx.execute<QueuePlayerRow>(
				`SELECT player_id, player_name, queued_at, conn_id FROM player_pool ORDER BY queued_at ASC LIMIT 2`,
			);
			if (rows.length < 2) return [];
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
	if (queued.length < 2) return;

	const a = queued[0]!;
	const b = queued[1]!;
	const matchId = crypto.randomUUID();
	const inviteCode = generateInviteCode();

	// Create the match outside the transaction, because a transaction must not
	// wait on another actor.
	const client = c.client<typeof registry>();
	try {
		await client.turnBasedMatch.create([matchId]);

		await client.turnBasedMatch.get([matchId]).createPlayer({
			playerId: a.player_id,
			playerName: a.player_name,
			symbol: "X" as const,
		});
		await client.turnBasedMatch.get([matchId]).createPlayer({
			playerId: b.player_id,
			playerName: b.player_name,
			symbol: "O" as const,
		});
	} catch (error) {
		await returnToPool(c, queued);
		throw error;
	}

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				`INSERT INTO matches (match_id, invite_code, player_count, is_open_pool, created_at) VALUES (?, ?, ?, ?, ?)`,
				matchId,
				inviteCode,
				2,
				1,
				Date.now(),
			);
			for (const player of queued) {
				await tx.execute(
					`INSERT INTO assignments (player_id, match_id, invite_code, conn_id) VALUES (?, ?, ?, ?)`,
					player.player_id,
					matchId,
					null,
					player.conn_id,
				);
			}
		},
		{ name: "save-match" },
	);

	for (const player of queued) {
		c.broadcast("assignmentReady", {
			matchId,
			playerId: player.player_id,
			connId: player.conn_id,
		} satisfies TurnBasedAssignment);
	}
}

/** Puts claimed players back at their original place in the pool. */
async function returnToPool(
	c: ActorContextOf<typeof turnBasedMatchmaker>,
	players: QueuePlayerRow[],
): Promise<void> {
	// A player who left while the match was being created stays out of the
	// pool, because their disconnect found no row to remove.
	const stillConnected = players.filter(
		(player) => player.conn_id !== null && c.conns.has(player.conn_id),
	);

	await c.db.transaction(
		async (tx) => {
			for (const player of stillConnected) {
				await tx.execute(
					`INSERT INTO player_pool (player_id, player_name, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
					player.player_id,
					player.player_name,
					player.queued_at,
					player.conn_id,
				);
			}
		},
		{ name: "return-players" },
	);
}
// docs:end queue

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			invite_code TEXT NOT NULL UNIQUE,
			player_count INTEGER NOT NULL,
			is_open_pool INTEGER NOT NULL DEFAULT 0,
			created_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_pool (
			player_id TEXT PRIMARY KEY,
			player_name TEXT NOT NULL,
			queued_at INTEGER NOT NULL,
			conn_id TEXT
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			invite_code TEXT,
			conn_id TEXT
		)
	`);
}
