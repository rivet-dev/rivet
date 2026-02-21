import { actor, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
import { registry } from "../index.ts";
import { generatePartyCode, generatePlayerName, MAX_PARTY_SIZE } from "./config.ts";

export const partyMatchmaker = actor({
	options: { name: "Party - Matchmaker", icon: "people-group" },
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
			(invoke.name === "createParty" || invoke.name === "joinParty")
		) {
			return !isInternal;
		}
		if (invoke.kind === "queue" && invoke.name === "closeParty") {
			return isInternal;
		}
		return false;
	},
	queues: {
		createParty: queue<
			{ hostName?: string },
			{ matchId: string; playerId: string; playerToken: string; partyCode: string }
		>(),
		joinParty: queue<
			{ partyCode: string; playerName?: string },
			{ matchId: string; playerId: string; playerToken: string }
		>(),
		closeParty: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "createParty") {
				const matchId = crypto.randomUUID();
				const partyCode = generatePartyCode();
				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();

				const client = c.client<typeof registry>();
				await client.partyMatch.create([matchId], {
					input: { matchId, partyCode },
				});

				await client.partyMatch
					.get([matchId], { params: { internalToken: INTERNAL_TOKEN } })
					.createPlayer({
						playerId,
						playerToken,
						playerName: message.body.hostName || generatePlayerName(),
						isHost: true,
					});

				await c.db.execute(
					`INSERT INTO parties (match_id, party_code, host_player_id, player_count, created_at) VALUES (?, ?, ?, ?, ?)`,
					matchId,
					partyCode,
					playerId,
					1,
					Date.now(),
				);

				await message.complete({ matchId, playerId, playerToken, partyCode });
			} else if (message.name === "joinParty") {
				const code = message.body.partyCode.toUpperCase().trim();
				const rows = await c.db.execute<{ match_id: string; player_count: number }>(
					`SELECT match_id, player_count FROM parties WHERE party_code = ?`,
					code,
				);
				const row = rows[0];
				if (!row) {
					throw new UserError("Party not found", { code: "party_not_found" });
				}
				if (row.player_count >= MAX_PARTY_SIZE) {
					throw new UserError("Party is full", { code: "party_full" });
				}

				const playerId = crypto.randomUUID();
				const playerToken = crypto.randomUUID();
				const client = c.client<typeof registry>();
				await client.partyMatch
					.get([row.match_id], { params: { internalToken: INTERNAL_TOKEN } })
					.createPlayer({
						playerId,
						playerToken,
						playerName: message.body.playerName || generatePlayerName(),
						isHost: false,
					});

				await c.db.execute(
					`UPDATE parties SET player_count = player_count + 1 WHERE match_id = ?`,
					row.match_id,
				);

				await message.complete({ matchId: row.match_id, playerId, playerToken });
			} else if (message.name === "closeParty") {
				await c.db.execute(
					`DELETE FROM parties WHERE match_id = ?`,
					message.body.matchId,
				);
				await message.complete();
			}
		}
	},
});

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS parties (
			match_id TEXT PRIMARY KEY,
			party_code TEXT NOT NULL UNIQUE,
			host_player_id TEXT NOT NULL,
			player_count INTEGER NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
}
