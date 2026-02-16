import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

const MIN_PLAYERS = 3;
const MAX_PLAYERS = 20;
const TICK_MS = 100;

interface QueueRow {
	player_id: string;
	queued_at: number;
}

interface AssignmentRow {
	player_id: string;
	match_id: string;
	assigned_at: number;
}

interface MatchRow {
	match_id: string;
}

interface MatchSeat {
	playerId: string;
	name: string;
}

export const battleRoyaleMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(["joinQueue", "matchClosed"], {
					count: 1,
					timeout: 100,
				})) ?? [];
			if (!message) continue;

			if (message.name === "joinQueue") {
				const input = message.body as { playerId?: string };
				if (!input?.playerId) {
					continue;
				}
				await processJoinQueue(c, { playerId: input.playerId });
				continue;
			}

			if (message.name === "matchClosed") {
				const input = message.body as { matchId?: string };
				if (!input?.matchId) {
					continue;
				}
				await processMatchClosed(c, {
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

async function processJoinQueue(c: MatchmakerContext, input: { playerId: string }) {
	const existing = await selectAssignment(c.db, input.playerId);
	if (existing) {
		return;
	}

	await enqueuePlayer(c.db, {
		playerId: input.playerId,
		queuedAt: Date.now(),
	});

	const createInput = await tryCreateMatch(c.db, input.playerId);
	if (createInput) {
		await createMatchActor(c, createInput);
	}
}

async function processMatchClosed(
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
	// This table stores queued battle royale players by enqueue time.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS queue (
			player_id TEXT PRIMARY KEY,
			queued_at INTEGER NOT NULL
		)
	`);
	// This table maps each queued player to a created battle royale match.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			assigned_at INTEGER NOT NULL
		)
	`);
	// This table stores created battle royale matches and initial player count.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			player_count INTEGER NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
	// This index speeds up oldest-first queue reads.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS queue_order_idx ON queue (queued_at)",
	);
}

async function selectAssignment(
	dbHandle: RawAccess,
	playerId: string,
): Promise<AssignmentRow | null> {
	// A player has at most one active battle royale assignment.
	const rows = (await dbHandle.execute(
		`SELECT player_id, match_id, assigned_at FROM assignments WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as AssignmentRow[];
	return rows[0] ?? null;
}

async function enqueuePlayer(
	dbHandle: RawAccess,
	input: { playerId: string; queuedAt: number },
) {
	// Enqueue idempotently so repeated calls do not duplicate rows.
	await dbHandle.execute(
		`INSERT OR IGNORE INTO queue (player_id, queued_at)
		VALUES (${sqlString(input.playerId)}, ${sqlInt(input.queuedAt)})`,
	);
}

async function tryCreateMatch(
	dbHandle: RawAccess,
	playerId: string,
): Promise<{ matchId: string; players: MatchSeat[] } | null> {
	await beginImmediateTransaction(dbHandle);
	try {
		// Re-read assignment under lock to avoid creating duplicate matches.
		const lockedAssignment = await selectAssignment(dbHandle, playerId);
		if (lockedAssignment) {
			await commitTransaction(dbHandle);
			return null;
		}

		// Build one match from the oldest queued players up to cap.
		const queueRows = await listQueueByOrder(dbHandle);
		if (queueRows.length < MIN_PLAYERS) {
			await commitTransaction(dbHandle);
			return null;
		}

		const selected = queueRows.slice(0, Math.min(queueRows.length, MAX_PLAYERS));
		const matchId = buildId("br");
		const now = Date.now();
		const players = selected.map((row) => ({
			playerId: row.player_id,
			name: row.player_id,
		}));
		await deleteQueuePlayers(
			dbHandle,
			selected.map((row) => row.player_id),
		);
		for (const player of players) {
			await insertAssignmentRow(dbHandle, {
				playerId: player.playerId,
				matchId,
				assignedAt: now,
			});
		}
		await insertMatchRow(dbHandle, {
			matchId,
			playerCount: selected.length,
			createdAt: now,
		});
		await commitTransaction(dbHandle);

		return {
			matchId,
			players,
		};
	} catch (err) {
		await rollbackTransaction(dbHandle);
		throw err;
	}
}

async function listQueueByOrder(dbHandle: RawAccess): Promise<QueueRow[]> {
	return (await dbHandle.execute(
		"SELECT player_id, queued_at FROM queue ORDER BY queued_at ASC",
	)) as QueueRow[];
}

async function countQueue(dbHandle: RawAccess): Promise<number> {
	const rows = (await dbHandle.execute("SELECT COUNT(*) AS count FROM queue")) as Array<{
		count: number;
	}>;
	return Number(rows[0]?.count ?? 0);
}

async function deleteQueuePlayers(dbHandle: RawAccess, playerIds: string[]) {
	if (playerIds.length === 0) {
		return;
	}
	const playerSql = playerIds.map((playerId) => sqlString(playerId)).join(", ");
	// Remove selected players from queue before writing assignments.
	await dbHandle.execute(`DELETE FROM queue WHERE player_id IN (${playerSql})`);
}

async function insertAssignmentRow(
	dbHandle: RawAccess,
	input: { playerId: string; matchId: string; assignedAt: number },
) {
	// Persist one assignment row per selected player.
	await dbHandle.execute(
		`INSERT INTO assignments (player_id, match_id, assigned_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.matchId)}, ${sqlInt(input.assignedAt)})`,
	);
}

async function insertMatchRow(
	dbHandle: RawAccess,
	input: { matchId: string; playerCount: number; createdAt: number },
) {
	// Persist match metadata for lifecycle and debugging.
	await dbHandle.execute(
		`INSERT INTO matches (match_id, player_count, created_at)
		VALUES (${sqlString(input.matchId)}, ${sqlInt(input.playerCount)}, ${sqlInt(input.createdAt)})`,
	);
}

async function selectMatchById(
	dbHandle: RawAccess,
	matchId: string,
): Promise<MatchRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT match_id FROM matches WHERE match_id = ${sqlString(matchId)} LIMIT 1`,
	)) as MatchRow[];
	return rows[0] ?? null;
}

async function deleteAssignmentsByMatchId(dbHandle: RawAccess, matchId: string) {
	await dbHandle.execute(`DELETE FROM assignments WHERE match_id = ${sqlString(matchId)}`);
}

async function deleteMatchById(dbHandle: RawAccess, matchId: string) {
	await dbHandle.execute(`DELETE FROM matches WHERE match_id = ${sqlString(matchId)}`);
}

async function beginImmediateTransaction(dbHandle: RawAccess) {
	await dbHandle.execute("BEGIN IMMEDIATE");
}

async function commitTransaction(dbHandle: RawAccess) {
	await dbHandle.execute("COMMIT");
}

async function rollbackTransaction(dbHandle: RawAccess) {
	await dbHandle.execute("ROLLBACK");
}

async function createMatchActor(
	c: MatchmakerContext,
	input: { matchId: string; players: MatchSeat[] },
) {
	// Create match actor after transaction commit.
	const client = c.client();
	await client.battleRoyaleMatch.create([input.matchId], {
		input: {
			matchId: input.matchId,
			tickMs: TICK_MS,
			players: input.players,
		},
	});
}

async function issuePlayerToken(
	c: MatchmakerContext,
	input: { matchId: string; playerId: string },
): Promise<string | null> {
	try {
		const client = c.client<any>();
		const result = (await client.battleRoyaleMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return result.playerToken ?? null;
	} catch {
		return null;
	}
}
