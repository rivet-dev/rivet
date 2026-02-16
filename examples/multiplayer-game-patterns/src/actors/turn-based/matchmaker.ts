import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

interface InviteRow {
	invite_code: string;
	from_player_id: string;
	to_player_id: string | null;
	status: string;
	match_id: string | null;
	created_at: number;
	updated_at: number;
}

interface AssignmentRow {
	player_id: string;
	match_id: string;
	assigned_at: number;
}

interface MatchRow {
	match_id: string;
}

interface QueueRow {
	player_id: string;
	queued_at: number;
}

interface MatchSeat {
	playerId: string;
	name: string;
}

interface CreateInput {
	matchId: string;
	players: [MatchSeat, MatchSeat];
}

export const asyncTurnBasedMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(
					["createInvite", "acceptInvite", "joinOpenPool", "matchCompleted"],
					{
						count: 1,
						timeout: 100,
					},
				)) ?? [];
			if (!message) continue;

			if (message.name === "createInvite") {
				const input = message.body as {
					inviteCode?: string;
					fromPlayerId?: string;
					toPlayerId?: string | null;
				};
				if (!input?.fromPlayerId) {
					continue;
				}
				await processCreateInvite(c, {
					inviteCode: input.inviteCode,
					fromPlayerId: input.fromPlayerId,
					toPlayerId: input.toPlayerId,
				});
				continue;
			}

			if (message.name === "acceptInvite") {
				const input = message.body as { inviteCode?: string; playerId?: string };
				if (!input?.inviteCode || !input?.playerId) {
					continue;
				}
				await processAcceptInvite(c, {
					inviteCode: input.inviteCode,
					playerId: input.playerId,
				});
				continue;
			}

			if (message.name === "joinOpenPool") {
				const input = message.body as { playerId?: string };
				if (!input?.playerId) {
					continue;
				}
				await processJoinOpenPool(c, { playerId: input.playerId });
				continue;
			}

			if (message.name === "matchCompleted") {
				const input = message.body as { matchId?: string };
				if (!input?.matchId) {
					continue;
				}
				await processMatchCompleted(c, {
					matchId: input.matchId,
				});
			}
		}
	},
	actions: {
		getAssignment: async (c, input: { playerId: string }) => {
			const assignment = await selectAssignment(c.db, input.playerId);
			if (!assignment) return null;
			const match = await selectMatchById(c.db, assignment.match_id);
			if (!match) return null;
			const playerToken = await issuePlayerToken(c, {
				matchId: assignment.match_id,
				playerId: assignment.player_id,
			});
			if (!playerToken) return null;
			return {
				playerId: assignment.player_id,
				matchId: assignment.match_id,
				playerToken,
				assignedAt: Number(assignment.assigned_at),
			};
		},
	},
});

type MatchmakerContext = {
	db: RawAccess;
	client: <T>() => any;
};

async function processCreateInvite(
	c: MatchmakerContext,
	input: { inviteCode?: string; fromPlayerId: string; toPlayerId?: string | null },
) {
	const inviteCode = input.inviteCode?.trim() || buildId("invite");
	const now = Date.now();
	await insertInvite(c.db, {
		inviteCode,
		fromPlayerId: input.fromPlayerId,
		toPlayerId: input.toPlayerId ?? "",
		createdAt: now,
		updatedAt: now,
	});
}

async function processAcceptInvite(
	c: MatchmakerContext,
	input: { inviteCode: string; playerId: string },
) {
	const invite = await selectInviteByCode(c.db, input.inviteCode);
	if (!invite) {
		return;
	}

	if (invite.status === "accepted" && invite.match_id) {
		return;
	}

	if (invite.status !== "open") {
		return;
	}

	if (
		invite.to_player_id &&
		invite.to_player_id.length > 0 &&
		invite.to_player_id !== input.playerId
	) {
		return;
	}

	const createInput = buildCreateInput("async", [invite.from_player_id, input.playerId]);
	await createMatch(c, {
		...createInput,
		source: "invite",
	});
	await markInviteAccepted(c.db, {
		inviteCode: input.inviteCode,
		matchId: createInput.matchId,
		updatedAt: Date.now(),
	});
}

async function processJoinOpenPool(c: MatchmakerContext, input: { playerId: string }) {
	const existing = await selectAssignment(c.db, input.playerId);
	if (existing) {
		return;
	}

	await enqueuePoolPlayer(c.db, {
		playerId: input.playerId,
		queuedAt: Date.now(),
	});

	const pending = await tryCreatePoolMatch(c.db, input.playerId);
	if (pending) {
		await createMatch(c, {
			...buildCreateInputFromExisting(pending.matchId, pending.playerIds),
			source: "open_pool",
		});
	}
}

async function processMatchCompleted(
	c: MatchmakerContext,
	input: { matchId: string },
) {
	const match = await selectMatchById(c.db, input.matchId);
	if (!match) {
		return;
	}
	await deleteAssignmentsByMatchId(c.db, input.matchId);
	await deleteMatchById(c.db, input.matchId);
}

async function migrateTables(dbHandle: RawAccess) {
	// This table stores invite metadata and acceptance status.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS invites (
			invite_code TEXT PRIMARY KEY,
			from_player_id TEXT NOT NULL,
			to_player_id TEXT,
			status TEXT NOT NULL,
			match_id TEXT,
			created_at INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	// This table is the open matchmaking pool for async players.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS pool_queue (
			player_id TEXT PRIMARY KEY,
			queued_at INTEGER NOT NULL
		)
	`);
	// This table maps players to the match they were assigned into.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			assigned_at INTEGER NOT NULL
		)
	`);
	// This table records created async matches and their origin.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			source TEXT NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
	// This index speeds up FIFO opponent lookup in the open pool.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS pool_queue_idx ON pool_queue (queued_at)",
	);
}

function buildCreateInput(prefix: string, playerIds: [string, string]): CreateInput {
	return buildCreateInputFromExisting(buildId(prefix), playerIds);
}

function buildCreateInputFromExisting(matchId: string, playerIds: [string, string]): CreateInput {
	const players = playerIds.map((playerId) => ({
		playerId,
		name: playerId,
	})) as [MatchSeat, MatchSeat];
	return {
		matchId,
		players,
	};
}

async function insertInvite(
	dbHandle: RawAccess,
	input: {
		inviteCode: string;
		fromPlayerId: string;
		toPlayerId: string;
		createdAt: number;
		updatedAt: number;
	},
) {
	// Create an invite row with open status until someone accepts it.
	await dbHandle.execute(
		`INSERT INTO invites (invite_code, from_player_id, to_player_id, status, match_id, created_at, updated_at)
		VALUES (${sqlString(input.inviteCode)}, ${sqlString(input.fromPlayerId)}, ${sqlString(input.toPlayerId)}, 'open', NULL, ${sqlInt(input.createdAt)}, ${sqlInt(input.updatedAt)})`,
	);
}

async function selectInviteByCode(
	dbHandle: RawAccess,
	inviteCode: string,
): Promise<InviteRow | null> {
	// Load one invite row for status and recipient checks.
	const rows = (await dbHandle.execute(
		`SELECT invite_code, from_player_id, to_player_id, status, match_id, created_at, updated_at FROM invites WHERE invite_code = ${sqlString(inviteCode)} LIMIT 1`,
	)) as InviteRow[];
	return rows[0] ?? null;
}

async function markInviteAccepted(
	dbHandle: RawAccess,
	input: { inviteCode: string; matchId: string; updatedAt: number },
) {
	// Persist acceptance so repeated calls return the same assignment.
	await dbHandle.execute(
		`UPDATE invites SET status = 'accepted', match_id = ${sqlString(input.matchId)}, updated_at = ${sqlInt(input.updatedAt)} WHERE invite_code = ${sqlString(input.inviteCode)}`,
	);
}

async function selectAssignment(
	dbHandle: RawAccess,
	playerId: string,
): Promise<AssignmentRow | null> {
	// A player has at most one active async assignment at a time.
	const rows = (await dbHandle.execute(
		`SELECT player_id, match_id, assigned_at FROM assignments WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as AssignmentRow[];
	return rows[0] ?? null;
}

async function enqueuePoolPlayer(
	dbHandle: RawAccess,
	input: { playerId: string; queuedAt: number },
) {
	// Enqueue idempotently for open-pool matchmaking.
	await dbHandle.execute(
		`INSERT OR IGNORE INTO pool_queue (player_id, queued_at)
		VALUES (${sqlString(input.playerId)}, ${sqlInt(input.queuedAt)})`,
	);
}

async function tryCreatePoolMatch(
	dbHandle: RawAccess,
	playerId: string,
): Promise<{ matchId: string; playerIds: [string, string] } | null> {
	return withImmediateTransaction(dbHandle, async () => {
		// Lock and recheck assignment to prevent double matching.
		const lockedAssignment = await selectAssignment(dbHandle, playerId);
		if (lockedAssignment) {
			return null;
		}

		// Pick the oldest waiting opponent.
		const opponent = await selectOldestPoolOpponent(dbHandle, playerId);
		if (!opponent) {
			return null;
		}

		const matchId = buildId("async-pool");
		await deletePoolPlayers(dbHandle, [opponent.player_id, playerId]);

		return {
			matchId,
			playerIds: [opponent.player_id, playerId],
		};
	});
}

async function selectOldestPoolOpponent(
	dbHandle: RawAccess,
	playerId: string,
): Promise<QueueRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT player_id, queued_at FROM pool_queue WHERE player_id != ${sqlString(playerId)} ORDER BY queued_at ASC LIMIT 1`,
	)) as QueueRow[];
	return rows[0] ?? null;
}

async function deletePoolPlayers(dbHandle: RawAccess, playerIds: string[]) {
	if (playerIds.length === 0) {
		return;
	}
	const playerSql = playerIds.map((id) => sqlString(id)).join(", ");
	// Remove both players from queue before creating the match.
	await dbHandle.execute(`DELETE FROM pool_queue WHERE player_id IN (${playerSql})`);
}

async function createMatch(
	c: MatchmakerContext,
	input: CreateInput & { source: "invite" | "open_pool" },
) {
	const now = Date.now();
	await insertMatchRow(c.db, {
		matchId: input.matchId,
		source: input.source,
		createdAt: now,
	});
	for (const player of input.players) {
		await upsertAssignment(c.db, {
			playerId: player.playerId,
			matchId: input.matchId,
			assignedAt: now,
		});
	}
	await createMatchActor(c, {
		matchId: input.matchId,
		players: input.players,
	});
}

async function insertMatchRow(
	dbHandle: RawAccess,
	input: {
		matchId: string;
		source: "invite" | "open_pool";
		createdAt: number;
	},
) {
	// Record the match source so operators can see whether it came from invite or pool.
	await dbHandle.execute(
		`INSERT INTO matches (match_id, source, created_at) VALUES (${sqlString(input.matchId)}, ${sqlString(input.source)}, ${sqlInt(input.createdAt)})`,
	);
}

async function upsertAssignment(
	dbHandle: RawAccess,
	input: { playerId: string; matchId: string; assignedAt: number },
) {
	// Upsert keeps one assignment row per player for polling and reconnect support.
	await dbHandle.execute(
		`INSERT INTO assignments (player_id, match_id, assigned_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.matchId)}, ${sqlInt(input.assignedAt)})
		ON CONFLICT(player_id) DO UPDATE SET
			match_id = excluded.match_id,
			assigned_at = excluded.assigned_at`,
	);
}

async function selectMatchById(dbHandle: RawAccess, matchId: string): Promise<MatchRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT match_id FROM matches WHERE match_id = ${sqlString(matchId)} LIMIT 1`,
	)) as MatchRow[];
	return rows[0] ?? null;
}

async function deleteAssignmentsByMatchId(dbHandle: RawAccess, matchId: string) {
	// Clear all active assignments for this finished match.
	await dbHandle.execute(`DELETE FROM assignments WHERE match_id = ${sqlString(matchId)}`);
}

async function deleteMatchById(dbHandle: RawAccess, matchId: string) {
	await dbHandle.execute(`DELETE FROM matches WHERE match_id = ${sqlString(matchId)}`);
}

async function withImmediateTransaction<T>(
	dbHandle: RawAccess,
	run: () => Promise<T>,
): Promise<T> {
	// Keep transaction control in one place so query flow reads top-to-bottom.
	await dbHandle.execute("BEGIN IMMEDIATE");
	try {
		const result = await run();
		await dbHandle.execute("COMMIT");
		return result;
	} catch (err) {
		await dbHandle.execute("ROLLBACK");
		throw err;
	}
}

async function createMatchActor(
	c: MatchmakerContext,
	input: { matchId: string; players: [MatchSeat, MatchSeat] },
) {
	// Create the match actor after SQL state is ready.
	const client = c.client();
	await client.asyncTurnBasedMatch.create([input.matchId], {
		input,
	});
}

async function issuePlayerToken(
	c: MatchmakerContext,
	input: { matchId: string; playerId: string },
): Promise<string | null> {
	try {
		const client = c.client<any>();
		const res = (await client.asyncTurnBasedMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return res.playerToken ?? null;
	} catch {
		return null;
	}
}
