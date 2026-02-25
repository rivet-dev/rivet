/*
This matchmaker uses a rating window flow.
1. queueForMatch adds players to player_pool with current rating and queue time.
2. attemptPairing scans the pool and looks for two players whose rating windows overlap.
3. createRankedMatch removes paired players, creates a match actor, stores assignments, and broadcasts assignmentReady.
4. matchCompleted updates player rating actors and leaderboard entries, then removes the match row.
*/
import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { interval } from "rivetkit/utils";

import { registry } from "../index.ts";
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

type QueuePlayerRow = {
	username: string;
	rating: number;
	queued_at: number;
	conn_id: string | null;
};

const QUEUE_UPDATE_TICK_MS = 1000;
const PAIRING_RETRY_TICK_MS = 2000;

export const rankedMatchmaker = actor({
	options: { name: "Ranked - Matchmaker", icon: "ranking-star" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		queueForMatch: queue<{
			username: string;
			connId: string;
		}>(),
		unqueueForMatch: queue<{ connId: string }>(),
		matchCompleted: queue<{
			matchId: string;
			winnerUsername: string;
			loserUsername: string;
			winnerNewRating: number;
			loserNewRating: number;
		}>(),
	},
	actions: {
		queueForMatch: async (c, { username }: { username: string }) => {
			const connId = c.conn.id;
			await c.queue.send("queueForMatch", {
				username,
				connId,
			});
			return { queued: true, connId };
		},
		getQueueSize: async (c) => {
			const rows = await c.db.execute<{ cnt: number }>(
				`SELECT COUNT(*) as cnt FROM player_pool`,
			);
			return rows[0]?.cnt ?? 0;
		},
		getAssignment: async (
			c,
			{ username }: { username: string },
		) => {
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
	},
	onDisconnect: async (c, conn) => {
		await c.queue.send("unqueueForMatch", { connId: conn.id });
	},
	run: async (c) => {
		c.waitUntil(runQueueUpdateTicker(c));

		while (!c.aborted) {
			const message = await c.queue.next({
				timeout: PAIRING_RETRY_TICK_MS,
			});
			if (!message) {
				// Retry pairing periodically so widening rating windows can form matches
				// even when no new queue messages arrive.
				await attemptPairing(c);
				continue;
			}

			if (message.name === "queueForMatch") {
				const { username, connId } = message.body;

				const client = c.client<typeof registry>();
				const playerHandle = client.rankedPlayer.getOrCreate([username]);
				await playerHandle.initialize({ username });
				const rating = await playerHandle.getRating() as number;

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
			} else if (message.name === "unqueueForMatch") {
				await c.db.execute(
					`DELETE FROM player_pool WHERE conn_id = ?`,
					message.body.connId,
				);
				await sendQueueUpdates(c);
			} else if (message.name === "matchCompleted") {
				const body = message.body;
				const client = c.client<typeof registry>();

				if (body.winnerUsername && body.loserUsername) {
					const winnerHandle = client.rankedPlayer.getOrCreate([body.winnerUsername]);
					await winnerHandle.applyMatchResult({ won: true, newRating: body.winnerNewRating });
					const loserHandle = client.rankedPlayer.getOrCreate([body.loserUsername]);
					await loserHandle.applyMatchResult({ won: false, newRating: body.loserNewRating });

					const winnerProfile = await winnerHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };
					const loserProfile = await loserHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };

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
			}
		}
	},
});

async function attemptPairing(
	c: ActorContextOf<typeof rankedMatchmaker>,
) {
	const now = Date.now();
	const pool = await c.db.execute<QueuePlayerRow>(
		`SELECT * FROM player_pool ORDER BY queued_at ASC`,
	);

	if (pool.length < 2) return;

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
			const b = pool[bestIdx]!;
			await createRankedMatch(c, a, b);
			return;
		}
	}
}

async function createRankedMatch(
	c: ActorContextOf<typeof rankedMatchmaker>,
	a: QueuePlayerRow,
	b: QueuePlayerRow,
) {
	await c.db.execute(`DELETE FROM player_pool WHERE username = ?`, a.username);
	await c.db.execute(`DELETE FROM player_pool WHERE username = ?`, b.username);

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

	const client = c.client<typeof registry>();
	await client.rankedMatch.create([matchId], {
		input: {
			matchId,
			assignedPlayers: assignedPlayers.map((p) => ({
				username: p.username,
				rating: p.rating,
			})),
		},
	});

	await c.db.execute(
		`INSERT INTO matches (match_id, created_at) VALUES (?, ?)`,
		matchId,
		Date.now(),
	);

	await sendQueueUpdates(c);

	for (const player of assignedPlayers) {
		await c.db.execute(
			`INSERT OR REPLACE INTO assignments (username, match_id, rating, conn_id) VALUES (?, ?, ?, ?)`,
			player.username,
			matchId,
			player.rating,
			player.connId,
		);
		c.broadcast("assignmentReady", {
			matchId,
			username: player.username,
			rating: player.rating,
			connId: player.connId,
		} satisfies RankedAssignment);
	}
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

async function runQueueUpdateTicker(c: ActorContextOf<typeof rankedMatchmaker>) {
	const tick = interval(QUEUE_UPDATE_TICK_MS);
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;
		await sendQueueUpdates(c);
	}
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
