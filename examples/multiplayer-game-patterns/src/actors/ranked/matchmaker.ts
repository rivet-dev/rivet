import { actor, type ActorContextOf, queue } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

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
}

export const rankedMatchmaker = actor({
	options: { name: "Ranked - Matchmaker", icon: "ranking-star" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		queueForMatch: queue<{ username: string }>(),
		matchCompleted: queue<{
			matchId: string;
			winnerUsername: string;
			loserUsername: string;
			winnerNewRating: number;
			loserNewRating: number;
		}>(),
	},
	actions: {
		registerPlayer: async (c, { username }: { username: string }) => {
			await c.db.execute(
				`UPDATE player_pool SET conn_id = ? WHERE username = ?`,
				c.conn.id,
				username,
			);
		},
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
			}>(
				`SELECT * FROM assignments WHERE username = ?`,
				username,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				username: row.username,
				rating: row.rating,
			};
		},
	},
	onDisconnect: async (c, conn) => {
		await c.db.execute(`DELETE FROM player_pool WHERE conn_id = ?`, conn.id);
		await broadcastQueueSize(c);
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "queueForMatch") {
				const { username } = message.body;

				// Ensure player actor exists and look up ELO.
				const client = c.client<typeof registry>();
				const playerHandle = client.rankedPlayer.getOrCreate([username]);
				await playerHandle.initialize({ username });
				const rating = await playerHandle.getRating() as number;

				await c.db.execute(
					`INSERT OR REPLACE INTO player_pool (username, rating, queued_at) VALUES (?, ?, ?)`,
					username,
					rating,
					Date.now(),
				);

				await broadcastQueueSize(c);
				await attemptPairing(c);
				await message.complete();
			} else if (message.name === "matchCompleted") {
				const body = message.body;
				const client = c.client<typeof registry>();

				if (body.winnerUsername && body.loserUsername) {
					// Update player actors with new ratings.
					const winnerHandle = client.rankedPlayer.getOrCreate([body.winnerUsername]);
					await winnerHandle.applyMatchResult({ won: true, newRating: body.winnerNewRating });
					const loserHandle = client.rankedPlayer.getOrCreate([body.loserUsername]);
					await loserHandle.applyMatchResult({ won: false, newRating: body.loserNewRating });

					// Fetch updated profiles for leaderboard.
					const winnerProfile = await winnerHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };
					const loserProfile = await loserHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };

					// Update leaderboard.
					const lb = client.rankedLeaderboard.getOrCreate(["main"]);
					await lb.updatePlayer(winnerProfile);
					await lb.updatePlayer(loserProfile);
				}

				await c.db.execute(
					`DELETE FROM matches WHERE match_id = ?`,
					body.matchId,
				);
				await message.complete();
			}
		}
	},
});

async function attemptPairing(
	c: ActorContextOf<typeof rankedMatchmaker>,
) {
	const now = Date.now();
	const pool = await c.db.execute<{
		username: string;
		rating: number;
		queued_at: number;
	}>(`SELECT * FROM player_pool ORDER BY queued_at ASC`);

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
	a: { username: string; rating: number },
	b: { username: string; rating: number },
) {
	await c.db.execute(`DELETE FROM player_pool WHERE username = ?`, a.username);
	await c.db.execute(`DELETE FROM player_pool WHERE username = ?`, b.username);

	const matchId = crypto.randomUUID();

	const client = c.client<typeof registry>();
	await client.rankedMatch.create([matchId], {
		input: {
			matchId,
			assignedPlayers: [
				{ username: a.username, rating: a.rating },
				{ username: b.username, rating: b.rating },
			],
		},
	});

	await c.db.execute(
		`INSERT INTO matches (match_id, created_at) VALUES (?, ?)`,
		matchId,
		Date.now(),
	);

	await broadcastQueueSize(c);

	// Store assignments so bots can poll for them.
	const assignments: RankedAssignment[] = [
		{ matchId, username: a.username, rating: a.rating },
		{ matchId, username: b.username, rating: b.rating },
	];
	for (const assignment of assignments) {
		await c.db.execute(
			`INSERT INTO assignments (username, match_id, rating) VALUES (?, ?, ?)`,
			assignment.username,
			assignment.matchId,
			assignment.rating,
		);
	}

	c.broadcast("assigned", { assignments });
}

async function broadcastQueueSize(c: ActorContextOf<typeof rankedMatchmaker>) {
	const rows = await c.db.execute<{ cnt: number }>(
		`SELECT COUNT(*) as cnt FROM player_pool`,
	);
	c.broadcast("queueUpdate", { count: rows[0]?.cnt ?? 0 });
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
			rating INTEGER NOT NULL
		)
	`);
}
