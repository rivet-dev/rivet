import { actor, type ActorContextOf, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
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
	playerToken: string;
	connId: string | null;
}

type QueuePlayerRow = {
	username: string;
	rating: number;
	queued_at: number;
	conn_id: string | null;
	registration_token: string | null;
};

type StoredRankedAssignment = RankedAssignment & {
	registrationToken: string | null;
};

export const rankedMatchmaker = actor({
	options: { name: "Ranked - Matchmaker", icon: "ranking-star" },
	db: db({
		onMigrate: migrateTables,
	}),
	onBeforeConnect: (_c, params: { internalToken?: string }) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		if (invoke.kind === "queue" && invoke.name === "queueForMatch") {
			return !isInternal;
		}
		if (invoke.kind === "queue" && invoke.name === "matchCompleted") {
			return isInternal;
		}
		if (
			invoke.kind === "action" &&
			(invoke.name === "registerPlayer" ||
				invoke.name === "getQueueSize" ||
				invoke.name === "getAssignment")
		) {
			return !isInternal;
		}
		if (invoke.kind === "subscribe" && invoke.name === "queueUpdate") {
			return !isInternal;
		}
		return false;
	},
	queues: {
		queueForMatch: queue<{ username: string }, { registrationToken: string }>(),
		matchCompleted: queue<{
			matchId: string;
			winnerUsername: string;
			loserUsername: string;
			winnerNewRating: number;
			loserNewRating: number;
		}>(),
	},
	actions: {
		registerPlayer: async (
			c,
			{ username, registrationToken }: { username: string; registrationToken: string },
		) => {
			await c.db.execute(
				`UPDATE player_pool SET conn_id = ? WHERE username = ? AND registration_token = ?`,
				c.conn.id,
				username,
				registrationToken,
			);
			const playerPoolChangeRows = await c.db.execute<{ changed: number }>(
				`SELECT changes() AS changed`,
			);
			const playerPoolChanges = playerPoolChangeRows[0]?.changed ?? 0;

			await c.db.execute(
				`UPDATE assignments SET conn_id = ? WHERE username = ? AND registration_token = ?`,
				c.conn.id,
				username,
				registrationToken,
			);
			const assignmentChangeRows = await c.db.execute<{ changed: number }>(
				`SELECT changes() AS changed`,
			);
			const assignmentChanges = assignmentChangeRows[0]?.changed ?? 0;

			if (playerPoolChanges === 0 && assignmentChanges === 0) {
				throw new UserError("forbidden", { code: "forbidden" });
			}
		},
		getQueueSize: async (c) => {
			const rows = await c.db.execute<{ cnt: number }>(
				`SELECT COUNT(*) as cnt FROM player_pool`,
			);
			return rows[0]?.cnt ?? 0;
		},
		getAssignment: async (
			c,
			{
				username,
				registrationToken,
			}: { username: string; registrationToken: string },
		) => {
			const rows = await c.db.execute<{
				match_id: string;
				username: string;
				rating: number;
				player_token: string;
				conn_id: string | null;
			}>(
				`SELECT * FROM assignments WHERE username = ? AND conn_id = ? AND registration_token = ?`,
				username,
				c.conn.id,
				registrationToken,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				username: row.username,
				rating: row.rating,
				playerToken: row.player_token,
				connId: row.conn_id,
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
				const registrationToken = crypto.randomUUID();

				// Ensure player actor exists and look up ELO.
				const client = c.client<typeof registry>();
				const playerHandle = client.rankedPlayer.getOrCreate([username], {
					params: { internalToken: INTERNAL_TOKEN },
				});
				await playerHandle.initialize({ username });
				const rating = await playerHandle.getRating() as number;

				await c.db.execute(
					`INSERT OR REPLACE INTO player_pool (username, rating, queued_at, registration_token) VALUES (?, ?, ?, ?)`,
					username,
					rating,
					Date.now(),
					registrationToken,
				);

				await broadcastQueueSize(c);
				await attemptPairing(c);
				await message.complete({ registrationToken });
			} else if (message.name === "matchCompleted") {
				const body = message.body;
				const client = c.client<typeof registry>();

				if (body.winnerUsername && body.loserUsername) {
					// Update player actors with new ratings.
					const winnerHandle = client.rankedPlayer.getOrCreate([body.winnerUsername], {
						params: { internalToken: INTERNAL_TOKEN },
					});
					await winnerHandle.applyMatchResult({ won: true, newRating: body.winnerNewRating });
					const loserHandle = client.rankedPlayer.getOrCreate([body.loserUsername], {
						params: { internalToken: INTERNAL_TOKEN },
					});
					await loserHandle.applyMatchResult({ won: false, newRating: body.loserNewRating });

					// Fetch updated profiles for leaderboard.
					const winnerProfile = await winnerHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };
					const loserProfile = await loserHandle.getProfile() as { username: string; rating: number; wins: number; losses: number };

					// Update leaderboard.
					const lb = client.rankedLeaderboard.getOrCreate(["main"], {
						params: { internalToken: INTERNAL_TOKEN },
					});
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
			token: crypto.randomUUID(),
			connId: a.conn_id,
			registrationToken: a.registration_token,
		},
		{
			username: b.username,
			rating: b.rating,
			token: crypto.randomUUID(),
			connId: b.conn_id,
			registrationToken: b.registration_token,
		},
	] as const;

	const client = c.client<typeof registry>();
	await client.rankedMatch.create([matchId], {
		input: {
			matchId,
			assignedPlayers: assignedPlayers.map((p) => ({
				username: p.username,
				rating: p.rating,
				token: p.token,
			})),
		},
	});

	await c.db.execute(
		`INSERT INTO matches (match_id, created_at) VALUES (?, ?)`,
		matchId,
		Date.now(),
	);

	await broadcastQueueSize(c);

	// Store assignments so clients can poll for them.
	const assignments: StoredRankedAssignment[] = assignedPlayers.map((player) => ({
		matchId,
		username: player.username,
		rating: player.rating,
		playerToken: player.token,
		connId: player.connId,
		registrationToken: player.registrationToken,
	}));
	for (const assignment of assignments) {
		await c.db.execute(
			`INSERT INTO assignments (username, match_id, rating, player_token, conn_id, registration_token) VALUES (?, ?, ?, ?, ?, ?)`,
			assignment.username,
			assignment.matchId,
			assignment.rating,
			assignment.playerToken,
			assignment.connId,
			assignment.registrationToken,
		);
	}
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
			conn_id TEXT,
			registration_token TEXT
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
			player_token TEXT NOT NULL,
			conn_id TEXT,
			registration_token TEXT
		)
	`);

	await ensureColumn(dbHandle, "player_pool", "registration_token", "TEXT");
	await ensureColumn(dbHandle, "assignments", "registration_token", "TEXT");
}

async function ensureColumn(
	dbHandle: RawAccess,
	table: "player_pool" | "assignments",
	column: "registration_token",
	definition: "TEXT",
) {
	const columns = await dbHandle.execute<{ name: string }>(
		`PRAGMA table_info(${table})`,
	);
	if (!columns.some((col) => col.name === column)) {
		await dbHandle.execute(
			`ALTER TABLE ${table} ADD COLUMN ${column} ${definition}`,
		);
	}
}
