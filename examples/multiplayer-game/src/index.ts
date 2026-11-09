import { actor, event, setup } from "rivetkit";
import { db } from "rivetkit/db";
import type { GameState, JoinResult, Player, RoomStats } from "./types.ts";

const MAX_PLAYERS = 10;
const WORLD_SIZE = 1200;
const START_MASS = 20;
const SPEED_BASE = 7;

const COLORS = [
	"#ff6b6b",
	"#6bc5ff",
	"#ffd36b",
	"#8affc1",
	"#b28dff",
	"#ff9ad5",
	"#7bffed",
	"#ffc48a",
];

const randomBetween = (min: number, max: number) =>
	Math.floor(Math.random() * (max - min + 1)) + min;

const radiusFromMass = (mass: number) => Math.max(8, Math.sqrt(mass) * 3.2);

const createPlayer = (id: string, name: string): Player => {
	const mass = START_MASS;
	return {
		id,
		name,
		x: randomBetween(80, WORLD_SIZE - 80),
		y: randomBetween(80, WORLD_SIZE - 80),
		color: COLORS[Math.floor(Math.random() * COLORS.length)],
		mass,
		radius: radiusFromMass(mass),
		lastUpdate: Date.now(),
	};
};

const buildGameState = (
	roomId: string,
	maxPlayers: number,
	players: Record<string, Player>,
): GameState => ({
	roomId,
	maxPlayers,
	players,
	updatedAt: Date.now(),
});

type GameRoomVars = {
	roomId: string;
	maxPlayers: number;
	players: Record<string, Player>;
	// Maps each connection ID to the player it controls.
	playerIds: Map<string, string>;
};

const ROOM_STATS_SELECT =
	"SELECT room_id AS roomId, player_count AS playerCount, created_at AS createdAt, last_updated_at AS lastUpdatedAt, max_players AS maxPlayers FROM rooms ORDER BY created_at";

const matchmaker = actor({
	// Coordinator actor that tracks active game rooms in SQLite. https://rivet.dev/docs/actors/design-patterns
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS rooms (
					room_id TEXT PRIMARY KEY,
					player_count INTEGER NOT NULL,
					created_at INTEGER NOT NULL,
					last_updated_at INTEGER NOT NULL,
					max_players INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		findGame: async (c) => {
			const openRooms = (await c.db.execute(
				"SELECT room_id AS roomId FROM rooms WHERE player_count < max_players ORDER BY created_at LIMIT 1",
			)) as { roomId: string }[];
			if (openRooms[0]) {
				return openRooms[0].roomId;
			}

			const [{ roomCount }] = (await c.db.execute(
				"SELECT COUNT(*) AS roomCount FROM rooms",
			)) as { roomCount: number }[];
			const roomId = `room-${roomCount + 1}`;
			const client = c.client<typeof registry>();
			await client.gameRoom.create([roomId], {
				input: { roomId, maxPlayers: MAX_PLAYERS },
			});

			const now = Date.now();
			await c.db.execute(
				"INSERT OR IGNORE INTO rooms (room_id, player_count, created_at, last_updated_at, max_players) VALUES (?, 0, ?, ?, ?)",
				roomId,
				now,
				now,
				MAX_PLAYERS,
			);

			return roomId;
		},
		listRooms: async (c) => {
			const rooms = (await c.db.execute(ROOM_STATS_SELECT)) as RoomStats[];
			return rooms.map((room) => room.roomId);
		},
		getRoomStats: async (c) => {
			return (await c.db.execute(ROOM_STATS_SELECT)) as RoomStats[];
		},
		updateRoomStats: async (c, roomId: string, playerCount: number) => {
			const now = Date.now();
			await c.db.execute(
				`INSERT INTO rooms (room_id, player_count, created_at, last_updated_at, max_players)
				VALUES (?, ?, ?, ?, ?)
				ON CONFLICT (room_id) DO UPDATE SET
					player_count = excluded.player_count,
					last_updated_at = excluded.last_updated_at`,
				roomId,
				playerCount,
				now,
				now,
				MAX_PLAYERS,
			);

			return { roomId, playerCount };
		},
	},
});

const gameRoom = actor({
	// Data actor that owns a room and broadcasts changes. The room settings are
	// saved in SQLite: https://rivet.dev/docs/actors/sqlite
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS room (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					room_id TEXT NOT NULL,
					max_players INTEGER NOT NULL
				)
			`);
		},
	}),
	onCreate: async (c, input: { roomId: string; maxPlayers: number }) => {
		await c.db.execute(
			"INSERT INTO room (id, room_id, max_players) VALUES (1, ?, ?)",
			input.roomId,
			input.maxPlayers,
		);
	},
	// Players only exist while they are connected, so they live in memory:
	// https://rivet.dev/docs/actors/state
	createVars: async (c): Promise<GameRoomVars> => {
		const [room] = (await c.db.execute(
			"SELECT room_id, max_players FROM room WHERE id = 1",
		)) as { room_id: string; max_players: number }[];
		return {
			roomId: room.room_id,
			maxPlayers: room.max_players,
			players: {},
			playerIds: new Map(),
		};
	},
	events: {
		playerLeft: event<{ playerId: string }>(),
		gameState: event<GameState>(),
		playerJoined: event<{ playerId: string; player: Player }>(),
		playerEaten: event<{
			eaterId: string;
			eatenId: string;
			eaterMass: number;
		}>(),
		playerMoved: event<{
			playerId: string;
			x: number;
			y: number;
			mass: number;
			radius: number;
		}>(),
	},
	onDisconnect: (c, conn) => {
		const playerId = c.vars.playerIds.get(conn.id);
		c.vars.playerIds.delete(conn.id);
		if (!playerId) return;

		if (c.vars.players[playerId]) {
			delete c.vars.players[playerId];
			c.broadcast("playerLeft", { playerId });
			c.broadcast(
				"gameState",
				buildGameState(
					c.vars.roomId,
					c.vars.maxPlayers,
					c.vars.players,
				),
			);
			void reportRoomCount(c);
		}
	},
	actions: {
		join: async (c, name: string): Promise<JoinResult | null> => {
			const existingId = c.vars.playerIds.get(c.conn.id);
			if (existingId && c.vars.players[existingId]) {
				return {
					playerId: existingId,
					player: c.vars.players[existingId],
				};
			}

			if (Object.keys(c.vars.players).length >= c.vars.maxPlayers) {
				return null;
			}

			const playerId = c.conn.id;
			const player = createPlayer(playerId, name);

			c.vars.playerIds.set(c.conn.id, playerId);
			c.vars.players[playerId] = player;

			c.broadcast("playerJoined", { playerId, player });
			c.broadcast(
				"gameState",
				buildGameState(
					c.vars.roomId,
					c.vars.maxPlayers,
					c.vars.players,
				),
			);
			await reportRoomCount(c);

			return { playerId, player };
		},
		move: async (c, dx: number, dy: number) => {
			const playerId = c.vars.playerIds.get(c.conn.id);
			if (!playerId) return null;

			const player = c.vars.players[playerId];
			if (!player) return null;

			const magnitude = Math.hypot(dx, dy);
			if (magnitude > 0) {
				const speed =
					SPEED_BASE * Math.max(0.4, START_MASS / player.mass);
				const nx = dx / magnitude;
				const ny = dy / magnitude;
				player.x = clamp(
					player.x + nx * speed,
					player.radius,
					WORLD_SIZE - player.radius,
				);
				player.y = clamp(
					player.y + ny * speed,
					player.radius,
					WORLD_SIZE - player.radius,
				);
				player.lastUpdate = Date.now();
			}

			const eaten = resolveCollisions(c.vars.players, c.vars.playerIds);
			for (const entry of eaten) {
				c.broadcast("playerEaten", entry);
			}

			if (c.vars.players[playerId]) {
				c.broadcast("playerMoved", {
					playerId,
					x: player.x,
					y: player.y,
					mass: player.mass,
					radius: player.radius,
				});
			}

			if (eaten.length > 0) {
				await reportRoomCount(c);
			}

			c.broadcast(
				"gameState",
				buildGameState(
					c.vars.roomId,
					c.vars.maxPlayers,
					c.vars.players,
				),
			);

			return {
				x: player.x,
				y: player.y,
				mass: player.mass,
				radius: player.radius,
			};
		},
		getState: (c) =>
			buildGameState(c.vars.roomId, c.vars.maxPlayers, c.vars.players),
	},
});

const clamp = (value: number, min: number, max: number) =>
	Math.max(min, Math.min(value, max));

const resolveCollisions = (
	players: Record<string, Player>,
	connPlayerIds: Map<string, string>,
) => {
	const playerIds = Object.keys(players);
	const ordered = playerIds.sort((a, b) => players[b].mass - players[a].mass);
	const removed = new Set<string>();
	const eaten: Array<{
		eaterId: string;
		eatenId: string;
		eaterMass: number;
	}> = [];

	for (let i = 0; i < ordered.length; i++) {
		const eaterId = ordered[i];
		if (removed.has(eaterId)) continue;
		const eater = players[eaterId];
		if (!eater) continue;

		for (let j = ordered.length - 1; j > i; j--) {
			const targetId = ordered[j];
			if (removed.has(targetId)) continue;
			const target = players[targetId];
			if (!target) continue;

			const distance = Math.hypot(eater.x - target.x, eater.y - target.y);
			if (distance < eater.radius && eater.mass > target.mass) {
				removed.add(targetId);
				eater.mass += target.mass;
				eater.radius = radiusFromMass(eater.mass);
				eaten.push({
					eaterId,
					eatenId: targetId,
					eaterMass: eater.mass,
				});
			}
		}
	}

	if (removed.size > 0) {
		for (const targetId of removed) {
			delete players[targetId];
			for (const [connId, playerId] of connPlayerIds) {
				if (playerId === targetId) {
					connPlayerIds.delete(connId);
				}
			}
		}
	}

	return eaten;
};

const reportRoomCount = async (c: {
	vars: GameRoomVars;
	client: <_T>() => any;
}) => {
	const client = c.client<typeof registry>();
	await client.matchmaker
		.getOrCreate(["main"])
		.updateRoomStats(c.vars.roomId, Object.keys(c.vars.players).length);
};

export const registry = setup({
	use: { matchmaker, gameRoom },
});

// Start the server on port 6420
registry.start();
