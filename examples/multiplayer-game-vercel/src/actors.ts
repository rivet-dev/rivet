import { actor, setup } from "rivetkit";
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

const buildGameState = (roomId: string, maxPlayers: number, players: Record<string, Player>): GameState => ({
	roomId,
	maxPlayers,
	players,
	updatedAt: Date.now(),
});

const matchmaker = actor({
	// Coordinator actor that tracks active game rooms. https://rivet.dev/docs/actors/design-patterns
	state: {
		rooms: [] as RoomStats[],
	},
	actions: {
		findGame: async (c) => {
			const openRoom = c.state.rooms.find(
				(room) => room.playerCount < room.maxPlayers,
			);
			if (openRoom) {
				return openRoom.roomId;
			}

			const roomId = `room-${c.state.rooms.length + 1}`;
			const client = c.client<typeof registry>();
			await client.gameRoom.create([roomId], {
				input: { roomId, maxPlayers: MAX_PLAYERS },
			});

			c.state.rooms.push({
				roomId,
				playerCount: 0,
				createdAt: Date.now(),
				lastUpdatedAt: Date.now(),
				maxPlayers: MAX_PLAYERS,
			});

			return roomId;
		},
		listRooms: (c) => c.state.rooms.map((room) => room.roomId),
		getRoomStats: (c) => c.state.rooms,
		updateRoomStats: (c, roomId: string, playerCount: number) => {
			const room = c.state.rooms.find((entry) => entry.roomId === roomId);
			if (room) {
				room.playerCount = playerCount;
				room.lastUpdatedAt = Date.now();
			} else {
				c.state.rooms.push({
					roomId,
					playerCount,
					createdAt: Date.now(),
					lastUpdatedAt: Date.now(),
					maxPlayers: MAX_PLAYERS,
				});
			}

			return { roomId, playerCount };
		},
	},
});

const gameRoom = actor({
	// Data actor that owns room state and broadcasts changes. https://rivet.dev/docs/actors/state
	createState: (_c, input: { roomId: string; maxPlayers: number }) => ({
		roomId: input.roomId,
		maxPlayers: input.maxPlayers,
		players: {} as Record<string, Player>,
	}),
	connState: {
		playerId: null as string | null,
	},
	onDisconnect: (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;

		if (c.state.players[playerId]) {
			delete c.state.players[playerId];
			c.broadcast("playerLeft", { playerId });
			c.broadcast("gameState", buildGameState(c.state.roomId, c.state.maxPlayers, c.state.players));
			void reportRoomCount(c);
		}

		conn.state.playerId = null;
	},
	actions: {
		join: async (c, name: string): Promise<JoinResult | null> => {
			const existingId = c.conn.state.playerId;
			if (existingId && c.state.players[existingId]) {
				return { playerId: existingId, player: c.state.players[existingId] };
			}

			if (Object.keys(c.state.players).length >= c.state.maxPlayers) {
				return null;
			}

			const playerId = c.conn.id;
			const player = createPlayer(playerId, name);

			c.conn.state.playerId = playerId;
			c.state.players[playerId] = player;

			c.broadcast("playerJoined", { playerId, player });
			c.broadcast("gameState", buildGameState(c.state.roomId, c.state.maxPlayers, c.state.players));
			await reportRoomCount(c);

			return { playerId, player };
		},
		move: async (c, dx: number, dy: number) => {
			const playerId = c.conn.state.playerId;
			if (!playerId) return null;

			const player = c.state.players[playerId];
			if (!player) return null;

			const magnitude = Math.hypot(dx, dy);
			if (magnitude > 0) {
				const speed = SPEED_BASE * Math.max(0.4, START_MASS / player.mass);
				const nx = dx / magnitude;
				const ny = dy / magnitude;
				player.x = clamp(player.x + nx * speed, player.radius, WORLD_SIZE - player.radius);
				player.y = clamp(player.y + ny * speed, player.radius, WORLD_SIZE - player.radius);
				player.lastUpdate = Date.now();
			}

			const eaten = resolveCollisions(c.state.players, c.conns);
			for (const entry of eaten) {
				c.broadcast("playerEaten", entry);
			}

			if (c.state.players[playerId]) {
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

			c.broadcast("gameState", buildGameState(c.state.roomId, c.state.maxPlayers, c.state.players));

			return { x: player.x, y: player.y, mass: player.mass, radius: player.radius };
		},
		getState: (c) => buildGameState(c.state.roomId, c.state.maxPlayers, c.state.players),
	},
});

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(value, max));

const resolveCollisions = (
	players: Record<string, Player>,
	connections: Map<string, { state: { playerId: string | null } }>,
) => {
	const playerIds = Object.keys(players);
	const ordered = playerIds.sort(
		(a, b) => players[b].mass - players[a].mass,
	);
	const removed = new Set<string>();
	const eaten: Array<{ eaterId: string; eatenId: string; eaterMass: number }> = [];

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
				eaten.push({ eaterId, eatenId: targetId, eaterMass: eater.mass });
			}
		}
	}

	if (removed.size > 0) {
		for (const targetId of removed) {
			delete players[targetId];
			for (const conn of connections.values()) {
				if (conn.state.playerId === targetId) {
					conn.state.playerId = null;
				}
			}
		}
	}

	return eaten;
};

const reportRoomCount = async (c: { state: { roomId: string; maxPlayers: number; players: Record<string, Player> }; client: <T>() => any }) => {
	const client = c.client<typeof registry>();
	await client.matchmaker
		.getOrCreate(["main"])
		.updateRoomStats(c.state.roomId, Object.keys(c.state.players).length);
};

export const registry = setup({
	use: { matchmaker, gameRoom },
});
