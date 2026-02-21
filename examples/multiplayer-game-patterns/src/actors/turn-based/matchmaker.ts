import { actor, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
import { registry } from "../index.ts";
import { generateInviteCode } from "./config.ts";

export const turnBasedMatchmaker = actor({
	options: { name: "Turn-Based - Matchmaker", icon: "chess-board" },
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
		if (
			invoke.kind === "queue" &&
			(invoke.name === "createGame" ||
				invoke.name === "joinByCode" ||
				invoke.name === "findMatch")
		) {
			return !isInternal;
		}
		if (invoke.kind === "queue" && invoke.name === "closeMatch") {
			return isInternal;
		}
		return false;
	},
	queues: {
		createGame: queue<
			{ playerName: string },
			{ matchId: string; playerId: string; playerToken: string; inviteCode: string }
		>(),
		joinByCode: queue<
			{ inviteCode: string; playerName: string },
			{ matchId: string; playerId: string; playerToken: string }
		>(),
		findMatch: queue<
			{ playerName: string },
			{ matchId: string; playerId: string; playerToken: string; inviteCode?: string }
		>(),
		closeMatch: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "createGame") {
				const matchId = crypto.randomUUID();
				const inviteCode = generateInviteCode();
				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();

				const client = c.client<typeof registry>();
				await client.turnBasedMatch.create([matchId], {
					input: { matchId },
				});

				await client.turnBasedMatch
					.get([matchId], { params: { internalToken: INTERNAL_TOKEN } })
					.createPlayer({
						playerId,
						playerToken,
						playerName: message.body.playerName,
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

				await message.complete({ matchId, playerId, playerToken, inviteCode });
			} else if (message.name === "joinByCode") {
				const code = message.body.inviteCode.toUpperCase().trim();
				const rows = await c.db.execute<{ match_id: string; player_count: number }>(
					`SELECT match_id, player_count FROM matches WHERE invite_code = ?`,
					code,
				);
				const row = rows[0];
				if (!row) throw new UserError("Game not found", { code: "game_not_found" });
				if (row.player_count >= 2) throw new UserError("Game is full", { code: "game_full" });

				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();
				const client = c.client<typeof registry>();
				await client.turnBasedMatch
					.get([row.match_id], { params: { internalToken: INTERNAL_TOKEN } })
					.createPlayer({
						playerId,
						playerToken,
						playerName: message.body.playerName,
						symbol: "O" as const,
					});

				await c.db.execute(
					`UPDATE matches SET player_count = 2 WHERE match_id = ?`,
					row.match_id,
				);

				await message.complete({ matchId: row.match_id, playerId, playerToken });
			} else if (message.name === "findMatch") {
				// Look for open pool game with 1 player.
				const rows = await c.db.execute<{ match_id: string }>(
					`SELECT match_id FROM matches WHERE is_open_pool = 1 AND player_count = 1 ORDER BY created_at ASC LIMIT 1`,
				);
				let matchId = rows[0]?.match_id ?? null;
				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();
				const client = c.client<typeof registry>();

				if (matchId) {
					// Join existing game as O.
					await client.turnBasedMatch
						.get([matchId], { params: { internalToken: INTERNAL_TOKEN } })
						.createPlayer({
							playerId,
							playerToken,
							playerName: message.body.playerName,
							symbol: "O" as const,
						});
					await c.db.execute(
						`UPDATE matches SET player_count = 2 WHERE match_id = ?`,
						matchId,
					);
				} else {
					// Create new open pool game.
					matchId = crypto.randomUUID();
					const inviteCode = generateInviteCode();
					await client.turnBasedMatch.create([matchId], {
						input: { matchId },
					});
					await client.turnBasedMatch
						.get([matchId], { params: { internalToken: INTERNAL_TOKEN } })
						.createPlayer({
							playerId,
							playerToken,
							playerName: message.body.playerName,
							symbol: "X" as const,
						});
					await c.db.execute(
						`INSERT INTO matches (match_id, invite_code, player_count, is_open_pool, created_at) VALUES (?, ?, ?, ?, ?)`,
						matchId,
						inviteCode,
						1,
						1,
						Date.now(),
					);
				}

				await message.complete({ matchId, playerId, playerToken });
			} else if (message.name === "closeMatch") {
				await c.db.execute(
					`DELETE FROM matches WHERE match_id = ?`,
					message.body.matchId,
				);
				await message.complete();
			}
		}
	},
});

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
}
