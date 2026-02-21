import { actor, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
import { registry } from "../index.ts";
import { LOBBY_CAPACITY } from "./config.ts";

export const battleRoyaleMatchmaker = actor({
	options: { name: "Battle Royale - Matchmaker", icon: "skull-crossbones" },
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
		if (invoke.kind === "queue" && invoke.name === "findMatch") {
			return !isInternal;
		}
		if (
			invoke.kind === "queue" &&
			(invoke.name === "updateMatch" || invoke.name === "closeMatch")
		) {
			return isInternal;
		}
		return false;
	},
	queues: {
		findMatch: queue<
			Record<string, never>,
			{ matchId: string; playerId: string; playerToken: string }
		>(),
		updateMatch: queue<{ matchId: string; playerCount: number; isStarted: boolean }>(),
		closeMatch: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "findMatch") {
				// Find a non-started match with room.
				const rows = await c.db.execute<{ match_id: string; player_count: number }>(
					`SELECT match_id, player_count FROM matches WHERE is_started = 0 AND player_count < ? ORDER BY player_count DESC, created_at ASC LIMIT 1`,
					LOBBY_CAPACITY,
				);
				let matchId = rows[0]?.match_id ?? null;

				if (!matchId) {
					matchId = crypto.randomUUID();
					await c.db.execute(
						`INSERT INTO matches (match_id, player_count, is_started, created_at) VALUES (?, ?, ?, ?)`,
						matchId,
						0,
						0,
						Date.now(),
					);
					const client = c.client<typeof registry>();
					await client.battleRoyaleMatch.create([matchId], {
						input: { matchId },
					});
				}

				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();
				const client = c.client<typeof registry>();
				await client.battleRoyaleMatch
					.get([matchId], { params: { internalToken: INTERNAL_TOKEN } })
					.createPlayer({ playerId, playerToken });

				await c.db.execute(
					`UPDATE matches SET player_count = player_count + 1 WHERE match_id = ?`,
					matchId,
				);

				await message.complete({ matchId, playerId, playerToken });
			} else if (message.name === "updateMatch") {
				await c.db.execute(
					`UPDATE matches SET player_count = ?, is_started = ? WHERE match_id = ?`,
					message.body.playerCount,
					message.body.isStarted ? 1 : 0,
					message.body.matchId,
				);
				await message.complete();
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
			player_count INTEGER NOT NULL,
			is_started INTEGER NOT NULL DEFAULT 0,
			created_at INTEGER NOT NULL
		)
	`);
}
