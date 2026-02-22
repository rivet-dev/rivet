/*
This matchmaker uses a private party flow with join tickets.
1. createParty creates a party actor and issues a host join ticket.
2. joinParty validates capacity and issues a join ticket for the party.
3. partyMatch verifies join tickets during createConnState before adding members.
4. partyMatch pushes connected member count updates through updatePartySize.
5. closeParty removes the party row and all join tickets.
*/
import { actor, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { registry } from "../index.ts";
import { generatePartyCode, generatePlayerName, MAX_PARTY_SIZE } from "./config.ts";

export const partyMatchmaker = actor({
	options: { name: "Party - Matchmaker", icon: "people-group" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		createParty: queue<
			{ hostName?: string },
			{
				matchId: string;
				playerId: string;
				partyCode: string;
				joinToken: string;
				playerName: string;
			}
		>(),
		joinParty: queue<
			{ partyCode: string; playerName?: string },
			{
				matchId: string;
				playerId: string;
				joinToken: string;
				playerName: string;
			}
		>(),
		verifyJoin: queue<
			{ matchId: string; playerId: string; joinToken: string },
			{ allowed: boolean; playerName?: string; isHost: boolean }
		>(),
		updatePartySize: queue<{ matchId: string; playerCount: number }>(),
		closeParty: queue<{ matchId: string }>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "createParty") {
				const now = Date.now();
				const matchId = crypto.randomUUID();
				const partyCode = generatePartyCode();
				const playerId = crypto.randomUUID();
				const joinToken = crypto.randomUUID();
				const playerName = message.body.hostName || generatePlayerName();

				const client = c.client<typeof registry>();
				await client.partyMatch.create([matchId], {
					input: { matchId, partyCode, hostPlayerId: playerId },
				});

				await c.db.execute(
					`INSERT INTO parties (match_id, party_code, host_player_id, player_count, created_at) VALUES (?, ?, ?, ?, ?)`,
					matchId,
					partyCode,
					playerId,
					1,
					now,
				);
				await c.db.execute(
					`INSERT INTO join_tickets (join_token, match_id, player_id, player_name, created_at) VALUES (?, ?, ?, ?, ?)`,
					joinToken,
					matchId,
					playerId,
					playerName,
					now,
				);

				await message.complete({
					matchId,
					playerId,
					partyCode,
					joinToken,
					playerName,
				});
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

				const now = Date.now();
				const playerId = crypto.randomUUID();
				const joinToken = crypto.randomUUID();
				const playerName = message.body.playerName || generatePlayerName();

				await c.db.execute(
					`UPDATE parties SET player_count = player_count + 1 WHERE match_id = ?`,
					row.match_id,
				);
				await c.db.execute(
					`INSERT INTO join_tickets (join_token, match_id, player_id, player_name, created_at) VALUES (?, ?, ?, ?, ?)`,
					joinToken,
					row.match_id,
					playerId,
					playerName,
					now,
				);

				await message.complete({
					matchId: row.match_id,
					playerId,
					joinToken,
					playerName,
				});
			} else if (message.name === "verifyJoin") {
				const rows = await c.db.execute<{
					player_name: string;
					host_player_id: string;
				}>(
					`SELECT jt.player_name, p.host_player_id
					 FROM join_tickets jt
					 INNER JOIN parties p ON p.match_id = jt.match_id
					 WHERE jt.join_token = ? AND jt.match_id = ? AND jt.player_id = ?
					 LIMIT 1`,
					message.body.joinToken,
					message.body.matchId,
					message.body.playerId,
				);
				const row = rows[0];
				if (!row) {
					await message.complete({ allowed: false, isHost: false });
					continue;
				}
				await message.complete({
					allowed: true,
					playerName: row.player_name,
					isHost: row.host_player_id === message.body.playerId,
				});
			} else if (message.name === "updatePartySize") {
				await c.db.execute(
					`UPDATE parties SET player_count = ? WHERE match_id = ?`,
					message.body.playerCount,
					message.body.matchId,
				);
				await message.complete();
			} else if (message.name === "closeParty") {
				await c.db.execute(
					`DELETE FROM join_tickets WHERE match_id = ?`,
					message.body.matchId,
				);
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
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS join_tickets (
			join_token TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			player_id TEXT NOT NULL,
			player_name TEXT NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS join_tickets_match_idx ON join_tickets (match_id)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS join_tickets_lookup_idx ON join_tickets (join_token, match_id, player_id)",
	);
}
