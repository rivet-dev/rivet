import { actor } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { buildId } from "../shared/ids.ts";
import { sqlInt, sqlString } from "../shared/sql.ts";

const DEFAULT_CAPACITY = 32;
const TICK_MS = 100;
const ROOM_STALE_MS = 5 * 60_000;

interface RoomRow {
	room_id: string;
	player_count: number;
	capacity: number;
	updated_at: number;
}

interface PlayerSessionRow {
	player_id: string;
	room_id: string;
	updated_at: number;
}

export const ioStyleMatchmaker = actor({
	db: db({
		onMigrate: migrateTables,
	}),
	run: async (c) => {
		while (!c.aborted) {
			const [message] =
				(await c.queue.next(["findOpenLobby", "roomHeartbeat", "roomClosed"], {
					count: 1,
					timeout: 100,
				})) ?? [];
			if (!message) continue;

			if (message.name === "findOpenLobby") {
				const input = message.body as { playerId?: string };
				if (!input?.playerId) {
					continue;
				}
				await processFindOpenLobby(c, { playerId: input.playerId });
				continue;
			}

			if (message.name === "roomHeartbeat") {
				const input = message.body as {
					matchId?: string;
					playerCount?: number;
					capacity?: number;
				};
				if (
					!input?.matchId ||
					typeof input.playerCount !== "number" ||
					typeof input.capacity !== "number"
				) {
					continue;
				}
				await processRoomHeartbeat(c, {
					matchId: input.matchId,
					playerCount: input.playerCount,
					capacity: input.capacity,
				});
				continue;
			}

			if (message.name === "roomClosed") {
				const input = message.body as { matchId?: string };
				if (!input?.matchId) {
					continue;
				}
				await processRoomClosed(c, {
					matchId: input.matchId,
				});
			}
		}
	},
	actions: {
		getLobbyForPlayer: async (c, input: { playerId: string }) => {
			const session = await selectPlayerSessionByPlayerId(c.db, input.playerId);
			if (!session) {
				return null;
			}
			const room = await selectRoomById(c.db, session.room_id);
			if (!room) {
				return null;
			}
			const playerToken = await issuePlayerToken(c, {
				matchId: room.room_id,
				playerId: session.player_id,
			});
			if (!playerToken) {
				return null;
			}
			return {
				matchId: room.room_id,
				playerId: session.player_id,
				playerToken,
				roomPlayerCount: Number(room.player_count),
				roomCapacity: Number(room.capacity),
			};
		},
	},
});

type MatchmakerContext = {
	db: RawAccess;
	client: <T>() => any;
};

async function processFindOpenLobby(c: MatchmakerContext, input: { playerId: string }) {
	const now = Date.now();
	await pruneStaleRooms(c.db, now);

	const existing = await selectPlayerSessionByPlayerId(c.db, input.playerId);
	if (existing) {
		const room = await selectRoomById(c.db, existing.room_id);
		if (room && Number(room.player_count) < Number(room.capacity)) {
			await touchRoom(c.db, room.room_id, now);
			return;
		}
		await deletePlayerSessionByPlayer(c.db, input.playerId);
	}

	let room = await selectBestOpenRoom(c.db);
	if (!room) {
		const matchId = buildId("io");
		await upsertRoom(c.db, {
			roomId: matchId,
			playerCount: 0,
			capacity: DEFAULT_CAPACITY,
			updatedAt: now,
		});
		await createMatchActor(c, { matchId });
		room = {
			room_id: matchId,
			player_count: 0,
			capacity: DEFAULT_CAPACITY,
			updated_at: now,
		};
	}

	await upsertPlayerSession(c.db, {
		playerId: input.playerId,
		roomId: room.room_id,
		updatedAt: now,
	});
	await touchRoom(c.db, room.room_id, now);
}

async function processRoomHeartbeat(
	c: MatchmakerContext,
	input: { matchId: string; playerCount: number; capacity: number },
) {
	const room = await selectRoomById(c.db, input.matchId);
	if (!room) {
		return;
	}
	await upsertRoom(c.db, {
		roomId: input.matchId,
		playerCount: input.playerCount,
		capacity: input.capacity,
		updatedAt: Date.now(),
	});
}

async function processRoomClosed(
	c: MatchmakerContext,
	input: { matchId: string },
) {
	const room = await selectRoomById(c.db, input.matchId);
	if (!room) {
		return;
	}
	await deletePlayerSessionsByRoom(c.db, input.matchId);
	await deleteRoom(c.db, input.matchId);
}

async function migrateTables(dbHandle: RawAccess) {
	// This table is the matchmaker index for io rooms.
	// Each row tracks occupancy and freshness for one room actor.
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS rooms (
			room_id TEXT PRIMARY KEY,
			player_count INTEGER NOT NULL,
			capacity INTEGER NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	await dbHandle.execute(`
		CREATE TABLE IF NOT EXISTS player_sessions (
			player_id TEXT PRIMARY KEY,
			room_id TEXT NOT NULL,
			updated_at INTEGER NOT NULL
		)
	`);
	// This index makes open-room lookups fast by occupancy and recency.
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS rooms_open_idx ON rooms (player_count, updated_at)",
	);
	await dbHandle.execute(
		"CREATE INDEX IF NOT EXISTS player_sessions_room_idx ON player_sessions (room_id)",
	);
}

async function pruneStaleRooms(dbHandle: RawAccess, now: number) {
	const cutoff = now - ROOM_STALE_MS;
	const staleRooms = (await dbHandle.execute(
		`SELECT room_id FROM rooms WHERE player_count = 0 AND updated_at < ${sqlInt(cutoff)}`,
	)) as Array<{ room_id: string }>;
	if (staleRooms.length === 0) {
		return;
	}
	const roomSql = staleRooms.map((row) => sqlString(row.room_id)).join(", ");
	// Remove empty rooms that have gone stale so the index stays clean.
	await dbHandle.execute(`DELETE FROM player_sessions WHERE room_id IN (${roomSql})`);
	await dbHandle.execute(`DELETE FROM rooms WHERE room_id IN (${roomSql})`);
}

async function selectRoomById(dbHandle: RawAccess, roomId: string): Promise<RoomRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT room_id, player_count, capacity, updated_at FROM rooms WHERE room_id = ${sqlString(roomId)} LIMIT 1`,
	)) as RoomRow[];
	return rows[0] ?? null;
}

async function selectBestOpenRoom(dbHandle: RawAccess): Promise<RoomRow | null> {
	// Route into the fullest open room so players converge quickly.
	const rows = (await dbHandle.execute(
		"SELECT room_id, player_count, capacity, updated_at FROM rooms WHERE player_count < capacity ORDER BY player_count DESC, updated_at DESC LIMIT 1",
	)) as RoomRow[];
	return rows[0] ?? null;
}

async function selectPlayerSessionByPlayerId(
	dbHandle: RawAccess,
	playerId: string,
): Promise<PlayerSessionRow | null> {
	const rows = (await dbHandle.execute(
		`SELECT player_id, room_id, updated_at FROM player_sessions WHERE player_id = ${sqlString(playerId)} LIMIT 1`,
	)) as PlayerSessionRow[];
	return rows[0] ?? null;
}

async function upsertPlayerSession(
	dbHandle: RawAccess,
	input: { playerId: string; roomId: string; updatedAt: number },
) {
	await dbHandle.execute(
		`INSERT INTO player_sessions (player_id, room_id, updated_at)
		VALUES (${sqlString(input.playerId)}, ${sqlString(input.roomId)}, ${sqlInt(input.updatedAt)})
		ON CONFLICT(player_id) DO UPDATE SET
			room_id = excluded.room_id,
			updated_at = excluded.updated_at`,
	);
}

async function deletePlayerSessionByPlayer(dbHandle: RawAccess, playerId: string) {
	await dbHandle.execute(`DELETE FROM player_sessions WHERE player_id = ${sqlString(playerId)}`);
}

async function deletePlayerSessionsByRoom(dbHandle: RawAccess, roomId: string) {
	await dbHandle.execute(`DELETE FROM player_sessions WHERE room_id = ${sqlString(roomId)}`);
}

async function touchRoom(dbHandle: RawAccess, roomId: string, now: number) {
	// Touch the row so the room is considered active during matchmaking.
	await dbHandle.execute(
		`UPDATE rooms SET updated_at = ${sqlInt(now)} WHERE room_id = ${sqlString(roomId)}`,
	);
}

async function upsertRoom(
	dbHandle: RawAccess,
	input: {
		roomId: string;
		playerCount: number;
		capacity: number;
		updatedAt: number;
	},
) {
	// Upsert keeps one canonical room row while live occupancy changes.
	await dbHandle.execute(
		`INSERT INTO rooms (room_id, player_count, capacity, updated_at)
		VALUES (${sqlString(input.roomId)}, ${sqlInt(input.playerCount)}, ${sqlInt(input.capacity)}, ${sqlInt(input.updatedAt)})
		ON CONFLICT(room_id) DO UPDATE SET
			player_count = excluded.player_count,
			capacity = excluded.capacity,
			updated_at = excluded.updated_at`,
	);
}

async function deleteRoom(dbHandle: RawAccess, roomId: string) {
	// Remove the room from matchmaking once the actor is closed.
	await dbHandle.execute(`DELETE FROM rooms WHERE room_id = ${sqlString(roomId)}`);
}

async function createMatchActor(
	c: MatchmakerContext,
	input: { matchId: string },
) {
	// Create a new room actor when no open room is available.
	const client = c.client();
	await client.ioStyleMatch.create([input.matchId], {
		input: {
			matchId: input.matchId,
			capacity: DEFAULT_CAPACITY,
			tickMs: TICK_MS,
		},
	});
}

async function issuePlayerToken(
	c: MatchmakerContext,
	input: { matchId: string; playerId: string },
): Promise<string | null> {
	try {
		const client = c.client<any>();
		const res = (await client.ioStyleMatch
			.get([input.matchId])
			.issuePlayerToken({
				playerId: input.playerId,
			})) as { playerToken?: string };
		return res.playerToken ?? null;
	} catch {
		return null;
	}
}
