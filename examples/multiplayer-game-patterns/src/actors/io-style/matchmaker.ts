import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { INTERNAL_TOKEN } from "../../auth.ts";
import { registry } from "../index.ts";
import { CAPACITY } from "./config.ts";

export const ioStyleMatchmaker = actor({
	options: { name: "IO - Matchmaker", icon: "earth-americas" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		// Sent by player
		findLobby: queue<Record<string, never>, { matchId: string; playerId: string; playerToken: string }>(),
		// Sent from match actor
		updateMatch: queue<{ matchId: string; playerCount: number }>(),
		// Sent from match actor
		closeMatch: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "findLobby") {
				const result = await processFindLobby(c);
				await message.complete(result);
			} else if (message.name === "updateMatch") {
				await c.db.execute(
					`UPDATE matches SET player_count = ?, updated_at = ? WHERE match_id = ?`,
					message.body.playerCount,
					Date.now(),
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

async function processFindLobby(
	c: ActorContextOf<typeof ioStyleMatchmaker>,
): Promise<{ matchId: string; playerId: string; playerToken: string }> {
	const rows = await c.db.execute<{ match_id: string; player_count: number }>(
		`SELECT match_id, player_count FROM matches WHERE player_count < ? ORDER BY player_count DESC, updated_at DESC LIMIT 1`,
		CAPACITY,
	);
	let matchId = rows[0]?.match_id ?? null;

	if (!matchId) {
		matchId = crypto.randomUUID();
		await c.db.execute(
			`INSERT INTO matches (match_id, player_count, updated_at) VALUES (?, ?, ?)`,
			matchId,
			0,
			Date.now(),
		);
		const client = c.client<typeof registry>();
		await client.ioStyleMatch.create([matchId], {
			input: { matchId },
		});
	}

	const playerId = crypto.randomUUID();
	const playerToken = crypto.randomUUID();
	const client = c.client<typeof registry>();
	await client.ioStyleMatch.get([matchId], { params: { internalToken: INTERNAL_TOKEN } }).createPlayer({
		playerId,
		playerToken,
	});
	await c.db.execute(
		`UPDATE matches SET player_count = player_count + 1, updated_at = ? WHERE match_id = ?`,
		Date.now(),
		matchId,
	);
	return { matchId, playerId, playerToken };
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			player_count INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS matches_open_idx ON matches (player_count, updated_at)",
	);
}
