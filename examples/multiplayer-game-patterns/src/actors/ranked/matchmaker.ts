/*
This matchmaker uses a rating window flow.
1. queueForMatch adds players to player_pool with current rating and queue time, then tries to pair them.
2. attemptPairing looks for two players whose rating windows overlap and removes both from the pool in one transaction.
3. createRankedMatch creates a match actor for the pair, stores assignments, and broadcasts assignmentReady.
4. A background loop retries pairing every few seconds, because rating windows widen while players wait.
5. matchCompleted updates player rating actors and leaderboard entries, then removes the match row.
*/
import { type ActorContextOf, actor, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import type { registry } from "../index.ts";
import {
	INITIAL_RATING_WINDOW,
	MAX_RATING_WINDOW,
	WINDOW_EXPAND_PER_SEC,
} from "./config.ts";

export interface RankedAssignment {
	matchId: string;
	username: string;
	rating: number;
	connId: string | null;
}

export type RankedQueueUpdate =
	| {
			queued: false;
			count: number;
	  }
	| {
			queued: true;
			count: number;
			username: string;
			rating: number;
			queueDurationMs: number;
			ratingWindow: number;
			ratingMin: number;
			ratingMax: number;
	  };

type QueuePlayerRow = {
	username: string;
	rating: number;
	queued_at: number;
	conn_id: string | null;
};

interface MatchmakerVars {
	// `onWake` stores its loops here so `onSleep` can wait for them.
	pairingRetryTicker: Promise<void> | null;
	queueUpdateTicker: Promise<void> | null;
}

const QUEUE_UPDATE_TICK_MS = 1000;
const PAIRING_RETRY_TICK_MS = 2000;

export const rankedMatchmaker = actor({
	options: { name: "Ranked - Matchmaker", icon: "ranking-star" },
	db: db({
		onMigrate: migrateTables,
	}),
	events: {
		assignmentReady: event<RankedAssignment>(),
		queueUpdate: event<RankedQueueUpdate>(),
	},
	createVars: (): MatchmakerVars => ({
		pairingRetryTicker: null,
		queueUpdateTicker: null,
	}),
	// docs:start retry
	// `onWake` runs on every start, so both loops restart after a sleep or crash.
	// They run separately, because a pairing attempt waits for a match actor to
	// be created and must not hold up the queue updates.
	onWake: (c) => {
		c.vars.pairingRetryTicker = runPairingRetryTicker(c).catch((error) => {
			c.log.error({ msg: "pairing retry loop failed", error });
		});
		c.vars.queueUpdateTicker = runQueueUpdateTicker(c).catch((error) => {
			c.log.error({ msg: "queue update loop failed", error });
		});
	},
	// `c.abortSignal` fires on shutdown, so both loops exit on their own
	onSleep: async (c) => {
		await Promise.all([
			c.vars.pairingRetryTicker,
			c.vars.queueUpdateTicker,
		]);
	},
	// docs:end retry
	actions: {
		// docs:start queue
		queueForMatch: async (c, { username }: { username: string }) => {
			const connId = c.conn.id;

			const client = c.client<typeof registry>();
			const playerHandle = client.rankedPlayer.getOrCreate([username]);
			await playerHandle.initialize({ username });
			const rating = await playerHandle.getRating();

			// Clear any stale assignment for this username before re-queueing.
			await c.db.execute(
				`DELETE FROM assignments WHERE username = ?`,
				username,
			);

			await c.db.execute(
				`INSERT OR REPLACE INTO player_pool (username, rating, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
				username,
				rating,
				Date.now(),
				connId,
			);

			await sendQueueUpdates(c);
			await attemptPairing(c);
			return { queued: true, connId };
		},
		// docs:end queue
		getQueueSize: async (c) => {
			const rows = await c.db.execute<{ cnt: number }>(
				`SELECT COUNT(*) as cnt FROM player_pool`,
			);
			return rows[0]?.cnt ?? 0;
		},
		getAssignment: async (c, { username }: { username: string }) => {
			const rows = await c.db.execute<{
				match_id: string;
				username: string;
				rating: number;
				conn_id: string | null;
			}>(
				`SELECT * FROM assignments WHERE username = ? AND conn_id = ?`,
				username,
				c.conn.id,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				username: row.username,
				rating: row.rating,
				connId: row.conn_id,
			};
		},
		// docs:start completed
		// Every step is safe to repeat: each player applies a match id once,
		// the leaderboard keeps the profile with the most games, and the
		// deletes do nothing the second time. A match can therefore send a
		// report again after one fails partway.
		matchCompleted: async (
			c,
			body: {
				matchId: string;
				winnerUsername: string;
				loserUsername: string;
				winnerNewRating: number;
				loserNewRating: number;
			},
		) => {
			const client = c.client<typeof registry>();

			if (body.winnerUsername && body.loserUsername) {
				const winnerHandle = client.rankedPlayer.getOrCreate([
					body.winnerUsername,
				]);
				await winnerHandle.applyMatchResult({
					matchId: body.matchId,
					won: true,
					newRating: body.winnerNewRating,
				});
				const loserHandle = client.rankedPlayer.getOrCreate([
					body.loserUsername,
				]);
				await loserHandle.applyMatchResult({
					matchId: body.matchId,
					won: false,
					newRating: body.loserNewRating,
				});

				const winnerProfile = await winnerHandle.getProfile();
				const loserProfile = await loserHandle.getProfile();

				const lb = client.rankedLeaderboard.getOrCreate(["main"]);
				await lb.updatePlayer(winnerProfile);
				await lb.updatePlayer(loserProfile);
			}

			await c.db.execute(
				`DELETE FROM matches WHERE match_id = ?`,
				body.matchId,
			);
			await c.db.execute(
				`DELETE FROM assignments WHERE match_id = ?`,
				body.matchId,
			);
		},
		// docs:end completed
	},
	onDisconnect: async (c, conn) => {
		await c.db.execute(
			`DELETE FROM player_pool WHERE conn_id = ?`,
			conn.id,
		);
		await sendQueueUpdates(c);
	},
});

// docs:start retry
/** Retries pairing on a timer, because rating windows widen while players wait. */
async function runPairingRetryTicker(
	c: ActorContextOf<typeof rankedMatchmaker>,
): Promise<void> {
	while (!c.aborted) {
		await delay(PAIRING_RETRY_TICK_MS, c.abortSignal);
		if (c.aborted) break;

		try {
			await attemptPairing(c);
		} catch (error) {
			// The pair is back in the pool, so the next tick tries again.
			c.log.warn({ msg: "pairing retry failed", error });
		}
	}
}
// docs:end retry

// docs:start pairing
async function attemptPairing(c: ActorContextOf<typeof rankedMatchmaker>) {
	// queueForMatch and the retry loop can pair at the same time, so pick the
	// pair and remove both players in one transaction. A player can't be
	// claimed by two matches.
	const pair = await c.db.transaction(
		async (tx) => {
			const pool = await tx.execute<QueuePlayerRow>(
				`SELECT * FROM player_pool ORDER BY queued_at ASC`,
			);
			const found = findPair(pool, Date.now());
			if (!found) return null;

			const [a, b] = found;
			await tx.execute(
				`DELETE FROM player_pool WHERE username IN (?, ?)`,
				a.username,
				b.username,
			);
			return found;
		},
		{ name: "claim-pair" },
	);
	if (!pair) return;

	const [a, b] = pair;
	await createRankedMatch(c, a, b);
}

/**
 * Returns the longest-waiting player and their closest opponent whose rating
 * windows both fit the difference, or null if no two players fit.
 */
function findPair(
	pool: QueuePlayerRow[],
	now: number,
): [QueuePlayerRow, QueuePlayerRow] | null {
	if (pool.length < 2) return null;

	for (let i = 0; i < pool.length; i++) {
		const a = pool[i]!;
		const aWaitSec = (now - a.queued_at) / 1000;
		const aWindow = Math.min(
			INITIAL_RATING_WINDOW + WINDOW_EXPAND_PER_SEC * aWaitSec,
			MAX_RATING_WINDOW,
		);

		let bestIdx = -1;
		let bestDiff = Infinity;

		for (let j = i + 1; j < pool.length; j++) {
			const b = pool[j]!;
			const bWaitSec = (now - b.queued_at) / 1000;
			const bWindow = Math.min(
				INITIAL_RATING_WINDOW + WINDOW_EXPAND_PER_SEC * bWaitSec,
				MAX_RATING_WINDOW,
			);

			const diff = Math.abs(a.rating - b.rating);
			if (diff <= aWindow && diff <= bWindow && diff < bestDiff) {
				bestDiff = diff;
				bestIdx = j;
			}
		}

		if (bestIdx >= 0) {
			return [a, pool[bestIdx]!];
		}
	}
	return null;
}
// docs:end pairing

async function createRankedMatch(
	c: ActorContextOf<typeof rankedMatchmaker>,
	a: QueuePlayerRow,
	b: QueuePlayerRow,
) {
	const matchId = crypto.randomUUID();
	const assignedPlayers = [
		{
			username: a.username,
			rating: a.rating,
			connId: a.conn_id,
		},
		{
			username: b.username,
			rating: b.rating,
			connId: b.conn_id,
		},
	] as const;

	// Create the match outside the claim transaction, because a transaction
	// must not wait on another actor.
	const client = c.client<typeof registry>();
	try {
		await client.rankedMatch.create([matchId], {
			input: {
				assignedPlayers: assignedPlayers.map((p) => ({
					username: p.username,
					rating: p.rating,
				})),
			},
		});
	} catch (error) {
		await returnToPool(c, [a, b]);
		throw error;
	}

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				`INSERT INTO matches (match_id, created_at) VALUES (?, ?)`,
				matchId,
				Date.now(),
			);
			for (const player of assignedPlayers) {
				await tx.execute(
					`INSERT OR REPLACE INTO assignments (username, match_id, rating, conn_id) VALUES (?, ?, ?, ?)`,
					player.username,
					matchId,
					player.rating,
					player.connId,
				);
			}
		},
		{ name: "save-match" },
	);

	await sendQueueUpdates(c);

	for (const player of assignedPlayers) {
		c.broadcast("assignmentReady", {
			matchId,
			username: player.username,
			rating: player.rating,
			connId: player.connId,
		} satisfies RankedAssignment);
	}
}

/** Puts claimed players back at their original place in the pool. */
async function returnToPool(
	c: ActorContextOf<typeof rankedMatchmaker>,
	players: QueuePlayerRow[],
) {
	// A player who left while the match was being created stays out of the
	// pool, because their disconnect found no row to remove.
	const stillConnected = players.filter(
		(player) => player.conn_id !== null && c.conns.has(player.conn_id),
	);

	await c.db.transaction(
		async (tx) => {
			for (const player of stillConnected) {
				// A player who queued again in the meantime keeps the newer entry.
				await tx.execute(
					`INSERT OR IGNORE INTO player_pool (username, rating, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
					player.username,
					player.rating,
					player.queued_at,
					player.conn_id,
				);
			}
		},
		{ name: "return-players" },
	);
	await sendQueueUpdates(c);
}

function calculateRatingWindow(now: number, queuedAt: number): number {
	const waitSec = Math.max(0, (now - queuedAt) / 1000);
	return Math.min(
		INITIAL_RATING_WINDOW + WINDOW_EXPAND_PER_SEC * waitSec,
		MAX_RATING_WINDOW,
	);
}

async function sendQueueUpdates(c: ActorContextOf<typeof rankedMatchmaker>) {
	if (c.conns.size === 0) return;

	const now = Date.now();
	const pool = await c.db.execute<QueuePlayerRow>(
		`SELECT * FROM player_pool`,
	);
	const count = pool.length;

	const playerByConnId = new Map<string, QueuePlayerRow>();
	for (const row of pool) {
		if (!row.conn_id) continue;
		const existing = playerByConnId.get(row.conn_id);
		if (!existing || row.queued_at > existing.queued_at) {
			playerByConnId.set(row.conn_id, row);
		}
	}

	for (const [connId, conn] of c.conns.entries()) {
		const player = playerByConnId.get(connId);
		if (!player) {
			conn.send("queueUpdate", {
				queued: false,
				count,
			});
			continue;
		}

		const ratingWindow = calculateRatingWindow(now, player.queued_at);
		conn.send("queueUpdate", {
			queued: true,
			count,
			username: player.username,
			rating: player.rating,
			queueDurationMs: Math.max(0, now - player.queued_at),
			ratingWindow: Math.round(ratingWindow),
			ratingMin: Math.round(player.rating - ratingWindow),
			ratingMax: Math.round(player.rating + ratingWindow),
		});
	}
}

async function runQueueUpdateTicker(
	c: ActorContextOf<typeof rankedMatchmaker>,
): Promise<void> {
	while (!c.aborted) {
		await delay(QUEUE_UPDATE_TICK_MS, c.abortSignal);
		if (c.aborted) break;
		await sendQueueUpdates(c);
	}
}

/** Resolves after `ms`, or as soon as the actor starts shutting down. */
function delay(ms: number, signal: AbortSignal): Promise<void> {
	return new Promise((resolve) => {
		if (signal.aborted) {
			resolve();
			return;
		}

		const finish = () => {
			clearTimeout(timer);
			signal.removeEventListener("abort", finish);
			resolve();
		};
		const timer = setTimeout(finish, ms);
		signal.addEventListener("abort", finish, { once: true });
	});
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_pool (
			username TEXT PRIMARY KEY,
			rating INTEGER NOT NULL,
			queued_at INTEGER NOT NULL,
			conn_id TEXT
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			created_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			username TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			rating INTEGER NOT NULL,
			conn_id TEXT
		)
	`);
}
