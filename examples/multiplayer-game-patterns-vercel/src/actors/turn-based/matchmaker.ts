/*
This matchmaker supports invite and queued public matchmaking flows.
1. createGame creates a private match with the first player as X.
2. joinByCode joins an existing private match by invite code as O.
3. queueForMatch action enqueues players into the public pool.
4. When two players are queued, the matchmaker creates a match and stores public assignments without invite codes.
5. getAssignment lets clients poll for their match assignment.
6. onDisconnect unqueues waiting players so stale queue entries do not remain.
*/
import { actor, type ActorContextOf, queue, UserError } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";

import { registry } from "../index.ts";
import { generateInviteCode } from "./config.ts";

export interface TurnBasedAssignment {
	matchId: string;
	playerId: string;
	inviteCode?: string;
	connId: string | null;
}

type QueuePlayerRow = {
	player_id: string;
	player_name: string;
	queued_at: number;
	conn_id: string | null;
};

export const turnBasedMatchmaker = actor({
	options: { name: "Turn-Based - Matchmaker", icon: "chess-board" },
	db: db({
		onMigrate: migrateTables,
	}),
	queues: {
		createGame: queue<
			{ playerName: string },
			{ matchId: string; playerId: string; inviteCode: string }
		>(),
		joinByCode: queue<
			{ inviteCode: string; playerName: string },
			{ matchId: string; playerId: string }
		>(),
		queueForMatch: queue<{
			playerId: string;
			playerName: string;
			connId: string;
		}>(),
		unqueueForMatch: queue<{ connId: string }>(),
		closeMatch: queue<{ matchId: string }>(),
	},
	actions: {
		queueForMatch: async (c, { playerName }: { playerName: string }) => {
			const playerId = crypto.randomUUID();
			await c.queue.send("queueForMatch", {
				playerId,
				playerName,
				connId: c.conn.id,
			});
			return { playerId };
		},
		getAssignment: async (c, { playerId }: { playerId: string }) => {
			const rows = await c.db.execute<{
				match_id: string;
				player_id: string;
				invite_code: string | null;
				conn_id: string | null;
			}>(
				`SELECT * FROM assignments WHERE player_id = ? AND conn_id = ?`,
				playerId,
				c.conn.id,
			);
			if (rows.length === 0) return null;
			const row = rows[0]!;
			return {
				matchId: row.match_id,
				playerId: row.player_id,
				inviteCode: row.invite_code ?? undefined,
				connId: row.conn_id,
			} satisfies TurnBasedAssignment;
		},
	},
	onDisconnect: async (c, conn) => {
		await c.queue.send("unqueueForMatch", { connId: conn.id });
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			if (message.name === "createGame") {
				const matchId = crypto.randomUUID();
				const inviteCode = generateInviteCode();
				const playerId = crypto.randomUUID();

				const client = c.client<typeof registry>();
				await client.turnBasedMatch.create([matchId], {
					input: { matchId },
				});

				await client.turnBasedMatch
					.get([matchId])
					.createPlayer({
						playerId,
						playerName: message.body.playerName,
						symbol: "X" as const,
					});

				await c.db.execute(
					`INSERT INTO matches (match_id, invite_code, player_count, is_open_pool, created_at) VALUES (?, ?, ?, ?, ?)`,
					matchId,
					inviteCode,
					1,
					0,
					Date.now(),
				);

				await message.complete({ matchId, playerId, inviteCode });
			} else if (message.name === "joinByCode") {
				const code = message.body.inviteCode.toUpperCase().trim();
				const rows = await c.db.execute<{ match_id: string; player_count: number }>(
					`SELECT match_id, player_count FROM matches WHERE invite_code = ?`,
					code,
				);
				const row = rows[0];
				if (!row) throw new UserError("Game not found", { code: "game_not_found" });
				if (row.player_count >= 2) throw new UserError("Game is full", { code: "game_full" });

				const playerId = crypto.randomUUID();
				const client = c.client<typeof registry>();
				await client.turnBasedMatch
					.get([row.match_id])
					.createPlayer({
						playerId,
						playerName: message.body.playerName,
						symbol: "O" as const,
					});

				await c.db.execute(
					`UPDATE matches SET player_count = 2 WHERE match_id = ?`,
					row.match_id,
				);

				await message.complete({ matchId: row.match_id, playerId });
			} else if (message.name === "queueForMatch") {
				await processQueueEntry(c, message.body);
				await message.complete();
			} else if (message.name === "unqueueForMatch") {
				await c.db.execute(
					`DELETE FROM player_pool WHERE conn_id = ?`,
					message.body.connId,
				);
				await message.complete();
			} else if (message.name === "closeMatch") {
				await c.db.execute(
					`DELETE FROM assignments WHERE match_id = ?`,
					message.body.matchId,
				);
				await c.db.execute(
					`DELETE FROM matches WHERE match_id = ?`,
					message.body.matchId,
				);
				await message.complete();
			}
		}
	},
});

async function processQueueEntry(
	c: ActorContextOf<typeof turnBasedMatchmaker>,
	entry: {
		playerId: string;
		playerName: string;
		connId: string;
	},
): Promise<void> {
	await c.db.execute(
		`INSERT OR REPLACE INTO player_pool (player_id, player_name, queued_at, conn_id) VALUES (?, ?, ?, ?)`,
		entry.playerId,
		entry.playerName,
		Date.now(),
		entry.connId,
	);

	await attemptPairing(c);
}

async function attemptPairing(
	c: ActorContextOf<typeof turnBasedMatchmaker>,
): Promise<void> {
	const queued = await c.db.execute<QueuePlayerRow>(
		`SELECT player_id, player_name, queued_at, conn_id FROM player_pool ORDER BY queued_at ASC LIMIT 2`,
	);
	if (queued.length < 2) return;

	const a = queued[0]!;
	const b = queued[1]!;

	await c.db.execute(`DELETE FROM player_pool WHERE player_id = ?`, a.player_id);
	await c.db.execute(`DELETE FROM player_pool WHERE player_id = ?`, b.player_id);

	const matchId = crypto.randomUUID();
	const inviteCode = generateInviteCode();

	const client = c.client<typeof registry>();
	await client.turnBasedMatch.create([matchId], {
		input: { matchId },
	});

	await client.turnBasedMatch
		.get([matchId])
		.createPlayer({
			playerId: a.player_id,
			playerName: a.player_name,
			symbol: "X" as const,
		});
	await client.turnBasedMatch
		.get([matchId])
		.createPlayer({
			playerId: b.player_id,
			playerName: b.player_name,
			symbol: "O" as const,
		});

	await c.db.execute(
		`INSERT INTO matches (match_id, invite_code, player_count, is_open_pool, created_at) VALUES (?, ?, ?, ?, ?)`,
		matchId,
		inviteCode,
		2,
		1,
		Date.now(),
	);

	const assignedPlayers = [a, b] as const;
	for (const player of assignedPlayers) {
		await c.db.execute(
			`INSERT INTO assignments (player_id, match_id, invite_code, conn_id) VALUES (?, ?, ?, ?)`,
			player.player_id,
			matchId,
			null,
			player.conn_id,
		);
		c.broadcast("assignmentReady", {
			matchId,
			playerId: player.player_id,
			connId: player.conn_id,
		} satisfies TurnBasedAssignment);
	}
}

async function migrateTables(dbHandle: RawAccess) {
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS matches (
			match_id TEXT PRIMARY KEY,
			invite_code TEXT NOT NULL UNIQUE,
			player_count INTEGER NOT NULL,
			is_open_pool INTEGER NOT NULL DEFAULT 0,
			created_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_pool (
			player_id TEXT PRIMARY KEY,
			player_name TEXT NOT NULL,
			queued_at INTEGER NOT NULL,
			conn_id TEXT
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS assignments (
			player_id TEXT PRIMARY KEY,
			match_id TEXT NOT NULL,
			invite_code TEXT,
			conn_id TEXT
		)
	`);
}
