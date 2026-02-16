import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId, buildPartyCode } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

interface RoomRow {
	party_code: string;
	match_id: string;
	host_player_id: string;
	status: string;
	created_at: number;
	updated_at: number;
}

interface MemberRow {
	party_code: string;
	player_id: string;
	joined_at: number;
}

export const partyMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(["createParty", "joinParty", "markStarted", "closeParty"], {
					count: 1,
					timeout: 100,
				})) ?? [];
			if (!message) continue;

			if (message.name === "createParty") {
				const input = message.body as { hostPlayerId?: string };
				if (!input?.hostPlayerId) {
					continue;
				}
				await processCreateParty(c, { hostPlayerId: input.hostPlayerId });
				continue;
			}

			if (message.name === "joinParty") {
				const input = message.body as { partyCode?: string; playerId?: string };
				if (!input?.partyCode || !input?.playerId) {
					continue;
				}
				await processJoinParty(c, {
					partyCode: input.partyCode,
					playerId: input.playerId,
				});
				continue;
			}

			if (message.name === "markStarted") {
				const input = message.body as { partyCode?: string };
				if (!input?.partyCode) {
					continue;
				}
				await processMarkStarted(c, {
					partyCode: input.partyCode,
				});
				continue;
			}

			if (message.name === "closeParty") {
				const input = message.body as { partyCode?: string };
				if (!input?.partyCode) {
					continue;
				}
				await processCloseParty(c, {
					partyCode: input.partyCode,
				});
			}
		}
	},
	actions: {
		getParty: async (c, input: { partyCode: string }) => {
			const partyCode = normalizeCode(input.partyCode);
			const room = await selectRoomByCode(c.db, partyCode);
			if (!room) return null;

			const members = await listMembersByJoinOrder(c.db, partyCode);
			return {
				partyCode: room.party_code,
				matchId: room.match_id,
				hostPlayerId: room.host_player_id,
				status: room.status,
				members: members.map((member) => ({
					playerId: member.player_id,
					joinedAt: Number(member.joined_at),
				})),
			};
		},
		getPartyForHost: async (c, input: { hostPlayerId: string }) => {
			const room = await selectLatestRoomByHost(c.db, input.hostPlayerId);
			if (!room) {
				return null;
			}
			const hostMember = await selectMemberByPlayer(c.db, room.party_code, room.host_player_id);
			if (!hostMember) {
				return null;
			}
			const hostPlayerToken = await issuePlayerToken(c, {
				matchId: room.match_id,
				playerId: hostMember.player_id,
			});
			if (!hostPlayerToken) {
				return null;
			}
			return {
				partyCode: room.party_code,
				matchId: room.match_id,
				hostPlayerId: room.host_player_id,
				hostPlayerToken,
				status: room.status,
			};
		},
		getJoinByPlayer: async (c, input: { partyCode: string; playerId: string }) => {
			const partyCode = normalizeCode(input.partyCode);
			const room = await selectRoomByCode(c.db, partyCode);
			if (!room) {
				return null;
			}
			const member = await selectMemberByPlayer(c.db, partyCode, input.playerId);
			if (!member) {
				return null;
			}
			const playerToken = await issuePlayerToken(c, {
				matchId: room.match_id,
				playerId: member.player_id,
			});
			if (!playerToken) {
				return null;
			}
			return {
				partyCode,
				matchId: room.match_id,
				playerId: member.player_id,
				playerToken,
			};
		},
	},
});

type MatchmakerContext = {
	db: RawAccess;
	client: <T>() => any;
};

async function processCreateParty(c: MatchmakerContext, input: { hostPlayerId: string }) {
	const now = Date.now();
	const matchId = buildId("party");
	const partyCode = await allocateCode(c.db);

	await insertRoom(c.db, {
		partyCode,
		matchId,
		hostPlayerId: input.hostPlayerId,
		status: "lobby",
		createdAt: now,
		updatedAt: now,
	});
	await upsertMember(c.db, {
		partyCode,
		playerId: input.hostPlayerId,
		joinedAt: now,
	});

	try {
		await createMatchActor(c, {
			matchId,
			partyCode,
			hostPlayerId: input.hostPlayerId,
		});
	} catch (err) {
		await deleteMembersByCode(c.db, partyCode);
		await deleteRoomByCode(c.db, partyCode);
		throw err;
	}
}

async function processJoinParty(
	c: MatchmakerContext,
	input: { partyCode: string; playerId: string },
) {
	const partyCode = normalizeCode(input.partyCode);
	const room = await selectRoomByCode(c.db, partyCode);
	if (!room) {
		return;
	}
	if (room.status !== "lobby") {
		return;
	}

	const existing = await selectMemberByPlayer(c.db, partyCode, input.playerId);
	if (existing) {
		await touchRoom(c.db, partyCode, Date.now());
		return;
	}

	await upsertMember(c.db, {
		partyCode,
		playerId: input.playerId,
		joinedAt: Date.now(),
	});
	await touchRoom(c.db, partyCode, Date.now());
}

async function processMarkStarted(
	c: MatchmakerContext,
	input: { partyCode: string },
) {
	const partyCode = normalizeCode(input.partyCode);
	const room = await selectRoomByCode(c.db, partyCode);
	if (!room) {
		return;
	}
	await updateRoomStatus(c.db, {
		partyCode,
		status: "in_progress",
		updatedAt: Date.now(),
	});
}

async function processCloseParty(
	c: MatchmakerContext,
	input: { partyCode: string },
) {
	const partyCode = normalizeCode(input.partyCode);
	const room = await selectRoomByCode(c.db, partyCode);
	if (!room) {
		return;
	}
	await deleteMembersByCode(c.db, partyCode);
	await deleteRoomByCode(c.db, partyCode);
}

function normalizeCode(code: string): string {
	return code.trim().toUpperCase();
}

async function migrateTables(dbHandle: RawAccess) {
	// This table stores one row per party room keyed by join code.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS rooms (
			party_code TEXT PRIMARY KEY,
			match_id TEXT NOT NULL UNIQUE,
			host_player_id TEXT NOT NULL,
			status TEXT NOT NULL,
			created_at INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	// This table stores party membership in join order.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS members (
			party_code TEXT NOT NULL,
			player_id TEXT NOT NULL,
			joined_at INTEGER NOT NULL,
			PRIMARY KEY (party_code, player_id)
		)
	`);
	// This index speeds up member list reads for lobby display.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS members_code_idx ON members (party_code, joined_at)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS members_lookup_idx ON members (party_code, player_id)",
	);
}

async function allocateCode(dbHandle: RawAccess): Promise<string> {
	for (let attempt = 0; attempt < 10; attempt++) {
		const nextCode = buildPartyCode();
		const existing = await selectRoomByCode(dbHandle, nextCode);
		if (!existing) {
			return nextCode;
		}
	}
	throw new Error("failed to allocate party code");
}

async function selectRoomByCode(
	dbHandle: RawAccess,
	partyCode: string,
): Promise<RoomRow | null> {
	// Party code maps to one room row.
	const rows = (await dbHandle.execute(
		`SELECT party_code, match_id, host_player_id, status, created_at, updated_at FROM rooms WHERE party_code = ${sqlString(partyCode)} LIMIT 1`,
	)) as RoomRow[];
	return rows[0] ?? null;
}

async function selectRoomByMatchId(
	dbHandle: RawAccess,
	matchId: string,
): Promise<RoomRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT party_code, match_id, host_player_id, status, created_at, updated_at FROM rooms WHERE match_id = ${sqlString(matchId)} LIMIT 1`,
	)) as RoomRow[];
	return rows[0] ?? null;
}

async function selectLatestRoomByHost(
	dbHandle: RawAccess,
	hostPlayerId: string,
): Promise<RoomRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT party_code, match_id, host_player_id, status, created_at, updated_at
		FROM rooms
		WHERE host_player_id = ${sqlString(hostPlayerId)}
		ORDER BY created_at DESC
		LIMIT 1`,
	)) as RoomRow[];
	return rows[0] ?? null;
}

async function insertRoom(
	dbHandle: RawAccess,
	input: {
		partyCode: string;
		matchId: string;
		hostPlayerId: string;
		status: "lobby" | "in_progress";
		createdAt: number;
		updatedAt: number;
	},
) {
	// Insert party metadata row for discovery and lifecycle status.
	await dbHandle.execute(
		`INSERT INTO rooms (party_code, match_id, host_player_id, status, created_at, updated_at)
		VALUES (${sqlString(input.partyCode)}, ${sqlString(input.matchId)}, ${sqlString(input.hostPlayerId)}, ${sqlString(input.status)}, ${sqlInt(input.createdAt)}, ${sqlInt(input.updatedAt)})`,
	);
}

async function selectMemberByPlayer(
	dbHandle: RawAccess,
	partyCode: string,
	playerId: string,
): Promise<MemberRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT party_code, player_id, joined_at FROM members WHERE party_code = ${sqlString(partyCode)} AND player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as MemberRow[];
	return rows[0] ?? null;
}

async function upsertMember(
	dbHandle: RawAccess,
	input: { partyCode: string; playerId: string; joinedAt: number },
) {
	// Upsert membership so reconnect flows keep the same token.
	await dbHandle.execute(
		`INSERT INTO members (party_code, player_id, joined_at)
		VALUES (${sqlString(input.partyCode)}, ${sqlString(input.playerId)}, ${sqlInt(input.joinedAt)})
		ON CONFLICT(party_code, player_id) DO UPDATE SET
			joined_at = excluded.joined_at`,
	);
}

async function touchRoom(dbHandle: RawAccess, partyCode: string, updatedAt: number) {
	// Touch the party row so operators can see recent activity.
	await dbHandle.execute(
		`UPDATE rooms SET updated_at = ${sqlInt(updatedAt)} WHERE party_code = ${sqlString(partyCode)}`,
	);
}

async function listMembersByJoinOrder(dbHandle: RawAccess, partyCode: string) {
	// Read members in join order for a predictable lobby roster.
	return (await dbHandle.execute(
		`SELECT player_id, joined_at FROM members WHERE party_code = ${sqlString(partyCode)} ORDER BY joined_at ASC`,
	)) as Array<{ player_id: string; joined_at: number }>;
}

async function updateRoomStatus(
	dbHandle: RawAccess,
	input: { partyCode: string; status: "in_progress"; updatedAt: number },
) {
	// Mark the party as started so late joins are rejected.
	await dbHandle.execute(
		`UPDATE rooms SET status = ${sqlString(input.status)}, updated_at = ${sqlInt(input.updatedAt)} WHERE party_code = ${sqlString(input.partyCode)}`,
	);
}

async function deleteMembersByCode(dbHandle: RawAccess, partyCode: string) {
	// Delete member rows before deleting room metadata.
	await dbHandle.execute(`DELETE FROM members WHERE party_code = ${sqlString(partyCode)}`);
}

async function deleteRoomByCode(dbHandle: RawAccess, partyCode: string) {
	await dbHandle.execute(`DELETE FROM rooms WHERE party_code = ${sqlString(partyCode)}`);
}

async function createMatchActor(
	c: MatchmakerContext,
	input: {
		matchId: string;
		partyCode: string;
		hostPlayerId: string;
	},
) {
	// Create the room actor before exposing it through matchmaker state.
	const client = c.client();
	await client.partyMatch.create([input.matchId], {
		input,
	});
}

async function issuePlayerToken(
	c: MatchmakerContext,
	input: { matchId: string; playerId: string },
): Promise<string | null> {
	try {
		const client = c.client<any>();
		const res = (await client.partyMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return res.playerToken ?? null;
	} catch {
		return null;
	}
}
