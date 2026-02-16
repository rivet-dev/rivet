import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

const DEFAULT_ELO = 1000;
const K_FACTOR = 24;
const BASE_WINDOW = 100;
const WINDOW_GROWTH_PER_SEC = 25;
const MAX_WINDOW = 400;
const TICK_MS = 50;

interface QueueRow {
	player_id: string;
	queued_at: number;
	elo: number;
}

interface AssignmentRow {
	player_id: string;
	match_id: string;
	opponent_player_id: string;
	assigned_at: number;
}

interface MatchRow {
	match_id: string;
	player_a: string;
	player_b: string;
	finished_at: number | null;
}

interface MatchSeat {
	playerId: string;
	name: string;
}

interface CreateRankedMatch {
	matchId: string;
	players: [MatchSeat, MatchSeat];
}

export const rankedMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(["debugSetRating", "queueForMatch", "reportResult"], {
					count: 1,
					timeout: 100,
				})) ?? [];
			if (!message) continue;

			if (message.name === "debugSetRating") {
				const input = message.body as { playerId?: string; elo?: number };
				if (!input?.playerId || typeof input.elo !== "number") {
					continue;
				}
				await processDebugSetRating(c, {
					playerId: input.playerId,
					elo: input.elo,
				});
				continue;
			}

			if (message.name === "queueForMatch") {
				const input = message.body as { playerId?: string };
				if (!input?.playerId) {
					continue;
				}
				await processQueueForMatch(c, { playerId: input.playerId });
				continue;
			}

			if (message.name === "reportResult") {
				const input = message.body as {
					matchId?: string;
					winnerPlayerId?: string | null;
				};
				if (!input?.matchId) {
					continue;
				}
				await processReportResult(c, {
					matchId: input.matchId,
					winnerPlayerId: input.winnerPlayerId,
				});
			}
		}
	},
	actions: {
		getRating: async (c, input: { playerId: string }) => {
			const row = await selectRating(c.db, input.playerId);
			return {
				playerId: input.playerId,
				elo: Number(row?.elo ?? DEFAULT_ELO),
				updatedAt: Number(row?.updated_at ?? Date.now()),
			};
		},
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
				opponentPlayerId: assignment.opponent_player_id,
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

async function processDebugSetRating(
	c: MatchmakerContext,
	input: { playerId: string; elo: number },
) {
	await upsertRating(c.db, {
		playerId: input.playerId,
		elo: input.elo,
		updatedAt: Date.now(),
	});
}

async function processQueueForMatch(c: MatchmakerContext, input: { playerId: string }) {
	await ensureRating(c.db, input.playerId);

	const assignment = await selectAssignment(c.db, input.playerId);
	if (assignment) {
		return;
	}

	const now = Date.now();
	await enqueuePlayer(c.db, {
		playerId: input.playerId,
		queuedAt: now,
	});

	const createInput = await tryCreateMatch(c.db, {
		playerId: input.playerId,
		now,
	});
	if (createInput) {
		await createMatchActor(c, createInput);
	}
}

async function processReportResult(
	c: MatchmakerContext,
	input: { matchId: string; winnerPlayerId?: string | null },
) {
	const match = await selectMatchById(c.db, input.matchId);
	if (!match) {
		return;
	}

	await ensureRating(c.db, match.player_a);
	await ensureRating(c.db, match.player_b);

	const ratings = await selectRatings(c.db, [match.player_a, match.player_b]);
	const eloA = ratings.get(match.player_a) ?? DEFAULT_ELO;
	const eloB = ratings.get(match.player_b) ?? DEFAULT_ELO;

	if (match.finished_at != null) {
		return;
	}

	if (
		input.winnerPlayerId &&
		input.winnerPlayerId !== match.player_a &&
		input.winnerPlayerId !== match.player_b
	) {
		return;
	}

	let scoreA = 0.5;
	let scoreB = 0.5;
	if (input.winnerPlayerId === match.player_a) {
		scoreA = 1;
		scoreB = 0;
	} else if (input.winnerPlayerId === match.player_b) {
		scoreA = 0;
		scoreB = 1;
	}

	const expectedA = expectedScore(eloA, eloB);
	const expectedB = expectedScore(eloB, eloA);
	const nextA = Math.round(eloA + K_FACTOR * (scoreA - expectedA));
	const nextB = Math.round(eloB + K_FACTOR * (scoreB - expectedB));

	const now = Date.now();
	await updateRating(c.db, {
		playerId: match.player_a,
		elo: nextA,
		updatedAt: now,
	});
	await updateRating(c.db, {
		playerId: match.player_b,
		elo: nextB,
		updatedAt: now,
	});
	await deleteAssignmentsByMatchId(c.db, input.matchId);
	await markMatchFinished(c.db, {
		matchId: input.matchId,
		finishedAt: now,
	});
}

function expectedScore(playerA: number, playerB: number) {
	return 1 / (1 + 10 ** ((playerB - playerA) / 400));
}

function computeSearchWindow(waitMs: number): number {
	return Math.min(
		MAX_WINDOW,
		BASE_WINDOW + Math.floor(waitMs / 1000) * WINDOW_GROWTH_PER_SEC,
	);
}

async function migrateTables(dbHandle: RawAccess) {
	// This table stores current ELO and update time per player.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS ratings (
			player_id TEXT PRIMARY KEY,
			elo INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	// This table stores ranked queue entries with enqueue timestamp.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS queue (
			player_id TEXT PRIMARY KEY,
			queued_at INTEGER NOT NULL
		)
	`);
	// This table maps each queued player to a created ranked match.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			opponent_player_id TEXT NOT NULL,
			assigned_at INTEGER NOT NULL
		)
	`);
	// This table records match pairings and completion timestamps.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			player_a TEXT NOT NULL,
			player_b TEXT NOT NULL,
			created_at INTEGER NOT NULL,
			finished_at INTEGER
		)
	`);
	// This index speeds up oldest-first queue scans.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS queue_queued_idx ON queue (queued_at)",
	);
}

async function ensureRating(dbHandle: RawAccess, playerId: string) {
	// Ensure every player has a rating row before matchmaking logic runs.
	await dbHandle.execute(
		`INSERT OR IGNORE INTO ratings (player_id, elo, updated_at)
		VALUES (${sqlString(playerId)}, ${sqlInt(DEFAULT_ELO)}, ${sqlInt(Date.now())})`,
	);
}

async function selectRating(
	dbHandle: RawAccess,
	playerId: string,
): Promise<{ elo: number; updated_at: number } | null> {
	// Read one rating row for display and debugging clients.
	const rows = (await dbHandle.execute(
		`SELECT elo, updated_at FROM ratings WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as Array<{ elo: number; updated_at: number }>;
	return rows[0] ?? null;
}

async function upsertRating(
	dbHandle: RawAccess,
	input: { playerId: string; elo: number; updatedAt: number },
) {
	// Upsert is used so tests can seed deterministic ELO values.
	await dbHandle.execute(
		`INSERT INTO ratings (player_id, elo, updated_at)
		VALUES (${sqlString(input.playerId)}, ${sqlInt(input.elo)}, ${sqlInt(input.updatedAt)})
		ON CONFLICT(player_id) DO UPDATE SET
			elo = excluded.elo,
			updated_at = excluded.updated_at`,
	);
}

async function selectAssignment(
	dbHandle: RawAccess,
	playerId: string,
): Promise<AssignmentRow | null> {
	// A player has at most one active ranked assignment.
	const rows = (await dbHandle.execute(
		`SELECT player_id, match_id, opponent_player_id, assigned_at FROM assignments WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as AssignmentRow[];
	return rows[0] ?? null;
}

async function enqueuePlayer(
	dbHandle: RawAccess,
	input: { playerId: string; queuedAt: number },
) {
	// Enqueue once per player. Repeated calls stay idempotent.
	await dbHandle.execute(
		`INSERT OR IGNORE INTO queue (player_id, queued_at) VALUES (${sqlString(input.playerId)}, ${sqlInt(input.queuedAt)})`,
	);
}

async function tryCreateMatch(
	dbHandle: RawAccess,
	input: { playerId: string; now: number },
): Promise<CreateRankedMatch | null> {
	await beginImmediateTransaction(dbHandle);
	try {
		// Re-read assignment under lock to prevent double pairing races.
		const lockedAssignment = await selectAssignment(dbHandle, input.playerId);
		if (lockedAssignment) {
			await commitTransaction(dbHandle);
			return null;
		}

		const self = await selectQueuedPlayerWithRating(dbHandle, input.playerId);
		if (!self) {
			await commitTransaction(dbHandle);
			return null;
		}

		const waitMs = input.now - Number(self.queued_at);
		const window = computeSearchWindow(waitMs);
		const opponent = await selectClosestOpponent(
			dbHandle,
			input.playerId,
			Number(self.elo),
			window,
		);
		if (!opponent) {
			await commitTransaction(dbHandle);
			return null;
		}

		const matchId = buildId("ranked");
		const firstSeat: MatchSeat = {
			playerId: input.playerId,
			name: input.playerId,
		};
		const secondSeat: MatchSeat = {
			playerId: opponent.player_id,
			name: opponent.player_id,
		};

		await deleteQueuePlayers(dbHandle, [input.playerId, opponent.player_id]);
		await insertMatchRow(dbHandle, {
			matchId,
			playerA: input.playerId,
			playerB: opponent.player_id,
			createdAt: input.now,
		});
		await insertAssignmentRow(dbHandle, {
			playerId: firstSeat.playerId,
			matchId,
			opponentPlayerId: secondSeat.playerId,
			assignedAt: input.now,
		});
		await insertAssignmentRow(dbHandle, {
			playerId: secondSeat.playerId,
			matchId,
			opponentPlayerId: firstSeat.playerId,
			assignedAt: input.now,
		});
		await commitTransaction(dbHandle);

		return {
			matchId,
			players: [firstSeat, secondSeat],
		};
	} catch (err) {
		await rollbackTransaction(dbHandle);
		throw err;
	}
}

async function selectQueuedPlayerWithRating(
	dbHandle: RawAccess,
	playerId: string,
): Promise<QueueRow | null> {
	// Load the queued player and current rating.
	const rows = (await dbHandle.execute(
		`SELECT q.player_id, q.queued_at, r.elo
		FROM queue q
		JOIN ratings r ON r.player_id = q.player_id
		WHERE q.player_id = ${sqlString(playerId)}
		LIMIT 1`,
	)) as QueueRow[];
	return rows[0] ?? null;
}

async function selectClosestOpponent(
	dbHandle: RawAccess,
	playerId: string,
	playerElo: number,
	window: number,
): Promise<QueueRow | null> {
	// Pick the closest-rated opponent still in queue.
	const rows = (await dbHandle.execute(
		`SELECT q.player_id, q.queued_at, r.elo
		FROM queue q
		JOIN ratings r ON r.player_id = q.player_id
		WHERE q.player_id != ${sqlString(playerId)}
			AND ABS(r.elo - ${sqlInt(playerElo)}) <= ${sqlInt(window)}
		ORDER BY ABS(r.elo - ${sqlInt(playerElo)}) ASC, q.queued_at ASC
		LIMIT 1`,
	)) as QueueRow[];
	return rows[0] ?? null;
}

async function deleteQueuePlayers(dbHandle: RawAccess, playerIds: string[]) {
	if (playerIds.length === 0) {
		return;
	}
	const playerSql = playerIds.map((playerId) => sqlString(playerId)).join(", ");
	// Remove paired players from queue before persisting assignments.
	await dbHandle.execute(`DELETE FROM queue WHERE player_id IN (${playerSql})`);
}

async function insertMatchRow(
	dbHandle: RawAccess,
	input: {
		matchId: string;
		playerA: string;
		playerB: string;
		createdAt: number;
	},
) {
	await dbHandle.execute(
		`INSERT INTO matches (match_id, player_a, player_b, created_at)
		VALUES (${sqlString(input.matchId)}, ${sqlString(input.playerA)}, ${sqlString(input.playerB)}, ${sqlInt(input.createdAt)})`,
	);
}

async function insertAssignmentRow(
	dbHandle: RawAccess,
	input: {
		playerId: string;
		matchId: string;
		opponentPlayerId: string;
		assignedAt: number;
	},
) {
	// Store per-player assignment rows for polling and reconnects.
	await dbHandle.execute(
		`INSERT INTO assignments (player_id, match_id, opponent_player_id, assigned_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.matchId)}, ${sqlString(input.opponentPlayerId)}, ${sqlInt(input.assignedAt)})`,
	);
}

async function createMatchActor(c: MatchmakerContext, input: CreateRankedMatch) {
	// Create the match actor after commit so SQL lock time stays short.
	const client = c.client();
	await client.rankedMatch.create([input.matchId], {
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
		const res = (await client.rankedMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return res.playerToken ?? null;
	} catch {
		return null;
	}
}

async function selectMatchById(
	dbHandle: RawAccess,
	matchId: string,
): Promise<MatchRow | null> {
	// Load the match pairing so ratings can be updated deterministically.
	const rows = (await dbHandle.execute(
		`SELECT match_id, player_a, player_b, finished_at FROM matches WHERE match_id = ${sqlString(matchId)} LIMIT 1`,
	)) as MatchRow[];
	return rows[0] ?? null;
}

async function selectRatings(dbHandle: RawAccess, playerIds: string[]): Promise<Map<string, number>> {
	if (playerIds.length === 0) {
		return new Map();
	}
	const playerSql = playerIds.map((playerId) => sqlString(playerId)).join(", ");
	// Read both ratings in one query to compute expected scores.
	const rows = (await dbHandle.execute(
		`SELECT player_id, elo FROM ratings WHERE player_id IN (${playerSql})`,
	)) as Array<{ player_id: string; elo: number }>;
	return new Map(rows.map((row) => [row.player_id, Number(row.elo)]));
}

async function updateRating(
	dbHandle: RawAccess,
	input: { playerId: string; elo: number; updatedAt: number },
) {
	await dbHandle.execute(
		`UPDATE ratings SET elo = ${sqlInt(input.elo)}, updated_at = ${sqlInt(input.updatedAt)} WHERE player_id = ${sqlString(input.playerId)}`,
	);
}

async function deleteAssignmentsByMatchId(dbHandle: RawAccess, matchId: string) {
	// Clear active assignments after match result has been applied.
	await dbHandle.execute(
		`DELETE FROM assignments WHERE match_id = ${sqlString(matchId)}`,
	);
}

async function markMatchFinished(
	dbHandle: RawAccess,
	input: { matchId: string; finishedAt: number },
) {
	await dbHandle.execute(
		`UPDATE matches SET finished_at = ${sqlInt(input.finishedAt)} WHERE match_id = ${sqlString(input.matchId)}`,
	);
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
