/*
This matchmaker uses a private party flow with join tickets.
1. createParty creates a party actor and issues a host join ticket.
2. joinParty validates capacity and issues a join ticket for the party.
3. partyMatch verifies join tickets in onBeforeConnect before adding members.
4. partyMatch pushes connected member count updates through updatePartySize.
5. closeParty removes the party row and all join tickets.
*/
import { actor, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import type { registry } from "../index.ts";
import {
	generatePartyCode,
	generatePlayerName,
	MAX_PARTY_SIZE,
} from "./config.ts";

export const partyMatchmaker = actor({
	options: { name: "Party - Matchmaker", icon: "people-group" },
	db: db({
		onMigrate: migrateTables,
	}),
	actions: {
		// docs:start create-join
		createParty: async (c, { hostName }: { hostName?: string }) => {
			const now = Date.now();
			const matchId = crypto.randomUUID();
			const partyCode = generatePartyCode();
			const playerId = crypto.randomUUID();
			const joinToken = crypto.randomUUID();
			const playerName = hostName || generatePlayerName();

			const client = c.client<typeof registry>();
			await client.partyMatch.create([matchId], {
				input: { partyCode },
			});

			// Save the party and the host's ticket together, so a party never
			// exists without a ticket for its host.
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						`INSERT INTO parties (match_id, party_code, host_player_id, player_count, created_at) VALUES (?, ?, ?, ?, ?)`,
						matchId,
						partyCode,
						playerId,
						1,
						now,
					);
					await tx.execute(
						`INSERT INTO join_tickets (join_token, match_id, player_id, player_name, created_at) VALUES (?, ?, ?, ?, ?)`,
						joinToken,
						matchId,
						playerId,
						playerName,
						now,
					);
				},
				{ name: "create-party" },
			);

			return { matchId, playerId, partyCode, joinToken, playerName };
		},
		joinParty: async (
			c,
			input: { partyCode: string; playerName?: string },
		) => {
			const code = input.partyCode.toUpperCase().trim();
			const now = Date.now();
			const playerId = crypto.randomUUID();
			const joinToken = crypto.randomUUID();
			const playerName = input.playerName || generatePlayerName();

			// Actions run concurrently, so check the size and take the slot in
			// one transaction. Two friends joining the last open slot at once
			// can't both get in.
			const matchId = await c.db.transaction(
				async (tx) => {
					const [party] = await tx.execute<{
						match_id: string;
						player_count: number;
					}>(
						`SELECT match_id, player_count FROM parties WHERE party_code = ?`,
						code,
					);
					if (!party) {
						throw new UserError("Party not found", {
							code: "party_not_found",
						});
					}
					if (party.player_count >= MAX_PARTY_SIZE) {
						throw new UserError("Party is full", {
							code: "party_full",
						});
					}

					await tx.execute(
						`UPDATE parties SET player_count = player_count + 1 WHERE match_id = ?`,
						party.match_id,
					);
					await tx.execute(
						`INSERT INTO join_tickets (join_token, match_id, player_id, player_name, created_at) VALUES (?, ?, ?, ?, ?)`,
						joinToken,
						party.match_id,
						playerId,
						playerName,
						now,
					);
					return party.match_id;
				},
				{ name: "join-party" },
			);

			return { matchId, playerId, joinToken, playerName };
		},
		// docs:end create-join
		// docs:start verify-join
		verifyJoin: async (
			c,
			input: { matchId: string; playerId: string; joinToken: string },
		) => {
			const [row] = await c.db.execute<{
				player_name: string;
				host_player_id: string;
			}>(
				`SELECT jt.player_name, p.host_player_id
				 FROM join_tickets jt
				 INNER JOIN parties p ON p.match_id = jt.match_id
				 WHERE jt.join_token = ? AND jt.match_id = ? AND jt.player_id = ?
				 LIMIT 1`,
				input.joinToken,
				input.matchId,
				input.playerId,
			);
			if (!row) {
				return { allowed: false, isHost: false };
			}
			return {
				allowed: true,
				playerName: row.player_name,
				isHost: row.host_player_id === input.playerId,
			};
		},
		// docs:end verify-join
		updatePartySize: async (
			c,
			{ matchId, playerCount }: { matchId: string; playerCount: number },
		) => {
			await c.db.execute(
				`UPDATE parties SET player_count = ? WHERE match_id = ?`,
				playerCount,
				matchId,
			);
		},
		closeParty: async (c, { matchId }: { matchId: string }) => {
			await c.db.transaction(
				async (tx) => {
					await tx.execute(
						`DELETE FROM join_tickets WHERE match_id = ?`,
						matchId,
					);
					await tx.execute(
						`DELETE FROM parties WHERE match_id = ?`,
						matchId,
					);
				},
				{ name: "close-party" },
			);
		},
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
