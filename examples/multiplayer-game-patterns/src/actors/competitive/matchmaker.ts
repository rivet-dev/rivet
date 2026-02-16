import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

export type Mode = "duo" | "squad";

const TICK_MS = 50;

const MODE_CONFIG: Record<Mode, { capacity: number; teams: number }> = {
	duo: { capacity: 4, teams: 2 },
	squad: { capacity: 8, teams: 2 },
};

interface AssignmentRow {
	player_id: string;
	match_id: string;
	mode: Mode;
	team_id: number;
	assigned_at: number;
}

interface MatchRow {
	match_id: string;
}

interface QueueRow {
	player_id: string;
	queued_at: number;
}

interface AssignedPlayer {
	playerId: string;
	name: string;
	teamId: number;
}

interface CreateMatchInput {
	matchId: string;
	mode: Mode;
	tickMs: number;
	capacity: number;
	assignedPlayers: AssignedPlayer[];
}

export const competitiveMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(["queueForMatch", "matchCompleted"], {
					count: 1,
					timeout: 100,
				})) ?? [];
			if (!message) continue;

			if (message.name === "queueForMatch") {
				const input = message.body as { playerId?: string; mode?: Mode };
				if (!input?.playerId || !input?.mode) {
					continue;
				}
				await processQueueForMatch(c, {
					playerId: input.playerId,
					mode: input.mode,
				});
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
				mode: assignment.mode,
				teamId: Number(assignment.team_id),
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

async function processQueueForMatch(
	c: MatchmakerContext,
	input: { playerId: string; mode: Mode },
) {
	const modeConfig = MODE_CONFIG[input.mode];
	if (!modeConfig) {
		return;
	}

	const existing = await selectAssignment(c.db, input.playerId);
	if (existing) {
		return;
	}

	const now = Date.now();
	await upsertQueueEntry(c.db, {
		playerId: input.playerId,
		mode: input.mode,
		queuedAt: now,
	});

	const createInput = await tryCreateMatch(c.db, {
		playerId: input.playerId,
		mode: input.mode,
		capacity: modeConfig.capacity,
		teams: modeConfig.teams,
		now,
	});

	if (createInput) {
		await createMatchActor(c, createInput);
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
	// This table stores waiting players grouped by mode and queue time.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS queue (
			player_id TEXT PRIMARY KEY,
			mode TEXT NOT NULL,
			queued_at INTEGER NOT NULL
		)
	`);
	// This table maps each queued player to the match and team they were assigned to.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			mode TEXT NOT NULL,
			team_id INTEGER NOT NULL,
			assigned_at INTEGER NOT NULL
		)
	`);
	// This table records created matches for lifecycle tracking and cleanup.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			mode TEXT NOT NULL,
			capacity INTEGER NOT NULL,
			created_at INTEGER NOT NULL
		)
	`);
	// This index speeds up queue reads by mode and arrival order.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS queue_mode_idx ON queue (mode, queued_at)",
	);
}

async function selectAssignment(
	dbHandle: RawAccess,
	playerId: string,
): Promise<AssignmentRow | null> {
	// A player has at most one active competitive assignment.
	const rows = (await dbHandle.execute(
		`SELECT player_id, match_id, mode, team_id, assigned_at FROM assignments WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as AssignmentRow[];
	return rows[0] ?? null;
}

async function upsertQueueEntry(
	dbHandle: RawAccess,
	input: { playerId: string; mode: Mode; queuedAt: number },
) {
	// Upsert keeps one queue row per player while allowing mode changes.
	await dbHandle.execute(
		`INSERT INTO queue (player_id, mode, queued_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.mode)}, ${sqlInt(input.queuedAt)})
		ON CONFLICT(player_id) DO UPDATE SET
			mode = excluded.mode,
			queued_at = excluded.queued_at`,
	);
}

async function tryCreateMatch(
	dbHandle: RawAccess,
	input: {
		playerId: string;
		mode: Mode;
		capacity: number;
		teams: number;
		now: number;
	},
): Promise<CreateMatchInput | null> {
	await beginImmediateTransaction(dbHandle);
	try {
		// Re-read assignment after locking to avoid double-match races.
		const lockedAssignment = await selectAssignment(dbHandle, input.playerId);
		if (lockedAssignment) {
			await commitTransaction(dbHandle);
			return null;
		}

		// Read the full queue for the requested mode in FIFO order.
		const queuedPlayers = await listQueueByMode(dbHandle, input.mode);
		if (queuedPlayers.length < input.capacity) {
			await commitTransaction(dbHandle);
			return null;
		}

		// Build one full match from the oldest players in this mode queue.
		const selected = queuedPlayers.slice(0, input.capacity);
		const matchId = buildId(`competitive-${input.mode}`);
		const assignedPlayers = selected.map((row, idx) => ({
			playerId: row.player_id,
			name: row.player_id,
			teamId: idx % input.teams,
		}));

		await deleteQueuePlayers(
			dbHandle,
			selected.map((row) => row.player_id),
		);
		await insertMatchRow(dbHandle, {
			matchId,
			mode: input.mode,
			capacity: input.capacity,
			createdAt: input.now,
		});
		for (const assigned of assignedPlayers) {
			await insertAssignmentRow(dbHandle, {
				playerId: assigned.playerId,
				matchId,
				mode: input.mode,
				teamId: assigned.teamId,
				assignedAt: input.now,
			});
		}

		await commitTransaction(dbHandle);
		return {
			matchId,
			mode: input.mode,
			tickMs: TICK_MS,
			capacity: input.capacity,
			assignedPlayers,
		};
	} catch (err) {
		await rollbackTransaction(dbHandle);
		throw err;
	}
}

async function listQueueByMode(
	dbHandle: RawAccess,
	mode: Mode,
): Promise<QueueRow[]> {
	return (await dbHandle.execute(
		`SELECT player_id, queued_at FROM queue WHERE mode = ${sqlString(mode)} ORDER BY queued_at ASC`,
	)) as QueueRow[];
}

async function countQueueByMode(
	dbHandle: RawAccess,
	mode: Mode,
): Promise<number> {
	const rows = (await dbHandle.execute(
		`SELECT COUNT(*) AS count FROM queue WHERE mode = ${sqlString(mode)}`,
	)) as Array<{ count: number }>;
	return Number(rows[0]?.count ?? 0);
}

async function deleteQueuePlayers(dbHandle: RawAccess, playerIds: string[]) {
	if (playerIds.length === 0) {
		return;
	}
	const selectedPlayerSql = playerIds.map((playerId) => sqlString(playerId)).join(", ");
	// Remove selected players from queue before persisting assignments.
	await dbHandle.execute(
		`DELETE FROM queue WHERE player_id IN (${selectedPlayerSql})`,
	);
}

async function insertMatchRow(
	dbHandle: RawAccess,
	input: {
		matchId: string;
		mode: Mode;
		capacity: number;
		createdAt: number;
	},
) {
	await dbHandle.execute(
		`INSERT INTO matches (match_id, mode, capacity, created_at) VALUES (${sqlString(input.matchId)}, ${sqlString(input.mode)}, ${sqlInt(input.capacity)}, ${sqlInt(input.createdAt)})`,
	);
}

async function insertAssignmentRow(
	dbHandle: RawAccess,
	input: {
		playerId: string;
		matchId: string;
		mode: Mode;
		teamId: number;
		assignedAt: number;
	},
) {
	// Persist a per-player assignment row so clients can poll their outcome.
	await dbHandle.execute(
		`INSERT INTO assignments (player_id, match_id, mode, team_id, assigned_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.matchId)}, ${sqlString(input.mode)}, ${sqlInt(input.teamId)}, ${sqlInt(input.assignedAt)})`,
	);
}

async function issuePlayerToken(
	c: MatchmakerContext,
	input: { matchId: string; playerId: string },
): Promise<string | null> {
	try {
		const client = c.client<any>();
		const result = (await client.competitiveMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return result.playerToken ?? null;
	} catch {
		return null;
	}
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
	// Remove assignments when the match actor reports completion.
	await dbHandle.execute(
		`DELETE FROM assignments WHERE match_id = ${sqlString(matchId)}`,
	);
}

async function deleteMatchById(dbHandle: RawAccess, matchId: string) {
	// Remove persisted match metadata after completion.
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

async function createMatchActor(c: MatchmakerContext, input: CreateMatchInput) {
	// Create the match actor after SQL commit so the lock stays short.
	const client = c.client();
	await client.competitiveMatch.create([input.matchId], {
		input,
	});
}
