import { type ActorContextOf, actor, setup } from "rivetkit";
import {
	WORLD_WIDTH,
	WORLD_HEIGHT,
	MIN_PLAYER_RADIUS,
	PLAYER_SPEED,
	TICK_RATE,
	PLAYER_COLORS,
	FOOD_COUNT,
	FOOD_RADIUS,
	FOOD_VALUE,
	MAX_LOBBY_SIZE,
	type Player,
	type Food,
	type GameStateEvent,
	type LobbyInfo,
} from "../shared/constants";

type GameState = {
	players: Record<string, Player>;
	food: Food[];
	nextFoodId: number;
};

// Matchmaker - assigns players to lobbies
export const matchmaker = actor({
	state: {
		lobbies: {} as Record<string, number>, // lobbyId -> playerCount
		nextLobbyId: 0,
	},

	actions: {
		findLobby: (c): LobbyInfo => {
			// Find a lobby with space
			for (const lobbyId in c.state.lobbies) {
				if (c.state.lobbies[lobbyId] < MAX_LOBBY_SIZE) {
					return { lobbyId };
				}
			}

			// No lobby with space, create new one
			const lobbyId = `lobby-${c.state.nextLobbyId++}`;
			c.state.lobbies[lobbyId] = 0;
			return { lobbyId };
		},

		setPlayerCount: (c, lobbyId: string, count: number) => {
			if (count <= 0) {
				delete c.state.lobbies[lobbyId];
			} else {
				c.state.lobbies[lobbyId] = count;
			}
		},
	},
});

// Game room - actual gameplay
export const gameRoom = actor({
	state: createInitialState(),

	createVars: () => {
		return { gameInterval: null as ReturnType<typeof setInterval> | null };
	},

	onWake: (c) => {
		c.vars.gameInterval = setInterval(() => {
			updateGame(c as ActorContextOf<typeof gameRoom>);
		}, TICK_RATE);
	},

	onSleep: (c) => {
		if (c.vars.gameInterval) {
			clearInterval(c.vars.gameInterval);
		}
	},

	onConnect: async (c, conn) => {
		// Spawn new player at random position
		const player: Player = {
			id: conn.id,
			x: Math.random() * (WORLD_WIDTH - 100) + 50,
			y: Math.random() * (WORLD_HEIGHT - 100) + 50,
			radius: MIN_PLAYER_RADIUS,
			color: PLAYER_COLORS[
				Math.floor(Math.random() * PLAYER_COLORS.length)
			],
			targetX: WORLD_WIDTH / 2,
			targetY: WORLD_HEIGHT / 2,
		};
		c.state.players[conn.id] = player;

		c.broadcast("playerJoined", { playerId: conn.id });

		// Notify matchmaker of player count change
		const lobbyId = c.key[0];
		const count = Object.keys(c.state.players).length;
		await c.client<typeof registry>().matchmaker.getOrCreate(["global"]).setPlayerCount(lobbyId, count);
	},

	onDisconnect: async (c, conn) => {
		delete c.state.players[conn.id];
		c.broadcast("playerLeft", { playerId: conn.id });

		// Notify matchmaker of player count change
		const lobbyId = c.key[0];
		const count = Object.keys(c.state.players).length;
		await c.client<typeof registry>().matchmaker.getOrCreate(["global"]).setPlayerCount(lobbyId, count);
	},

	actions: {
		setTarget: (c, targetX: number, targetY: number) => {
			const player = c.state.players[c.conn.id];
			if (player) {
				player.targetX = targetX;
				player.targetY = targetY;
			}
		},

		getState: (c): GameStateEvent => ({
			players: Object.values(c.state.players),
			food: c.state.food,
		}),

		getPlayerId: (c): string => {
			return c.conn.id;
		},

		getPlayerCount: (c): number => {
			return Object.keys(c.state.players).length;
		},
	},
});

function createInitialState(): GameState {
	const food: Food[] = [];
	for (let i = 0; i < FOOD_COUNT; i++) {
		food.push(createFood(i));
	}
	return {
		players: {},
		food,
		nextFoodId: FOOD_COUNT,
	};
}

function updateGame(c: ActorContextOf<typeof gameRoom>) {
	const players = Object.values(c.state.players);

	// Update player positions - move towards target
	for (const player of players) {
		const dx = player.targetX - player.x;
		const dy = player.targetY - player.y;
		const distance = Math.sqrt(dx * dx + dy * dy);

		if (distance > 5) {
			// Speed decreases as player gets larger
			const speed = PLAYER_SPEED * (MIN_PLAYER_RADIUS / player.radius);
			const moveX = (dx / distance) * speed;
			const moveY = (dy / distance) * speed;

			player.x = Math.max(
				player.radius,
				Math.min(WORLD_WIDTH - player.radius, player.x + moveX),
			);
			player.y = Math.max(
				player.radius,
				Math.min(WORLD_HEIGHT - player.radius, player.y + moveY),
			);
		}
	}

	// Check for food collisions
	for (const player of players) {
		for (let i = c.state.food.length - 1; i >= 0; i--) {
			const food = c.state.food[i];
			const dx = food.x - player.x;
			const dy = food.y - player.y;
			const distance = Math.sqrt(dx * dx + dy * dy);

			if (distance < player.radius + FOOD_RADIUS) {
				// Eat the food
				player.radius += FOOD_VALUE;
				// Replace with new food
				c.state.food[i] = createFood(c.state.nextFoodId++);
			}
		}
	}

	// Check for collisions between players
	for (let i = 0; i < players.length; i++) {
		for (let j = i + 1; j < players.length; j++) {
			const p1 = players[i];
			const p2 = players[j];

			const dx = p2.x - p1.x;
			const dy = p2.y - p1.y;
			const distance = Math.sqrt(dx * dx + dy * dy);

			// Check if circles overlap significantly (one center inside the other)
			if (distance < Math.max(p1.radius, p2.radius)) {
				// Larger player eats smaller player
				if (p1.radius > p2.radius * 1.1) {
					// Must be 10% bigger to eat
					// Absorb mass (area-based)
					p1.radius = Math.sqrt(
						p1.radius * p1.radius + p2.radius * p2.radius,
					);
					// Respawn eaten player
					respawnPlayer(p2);
				} else if (p2.radius > p1.radius * 1.1) {
					p2.radius = Math.sqrt(
						p2.radius * p2.radius + p1.radius * p1.radius,
					);
					respawnPlayer(p1);
				}
			}
		}
	}

	// Broadcast game state to all connected clients
	c.broadcast("gameState", {
		players: players,
		food: c.state.food,
	} satisfies GameStateEvent);
}

function createFood(id: number): Food {
	return {
		id,
		x: Math.random() * WORLD_WIDTH,
		y: Math.random() * WORLD_HEIGHT,
		color: PLAYER_COLORS[Math.floor(Math.random() * PLAYER_COLORS.length)],
	};
}

function respawnPlayer(player: Player) {
	player.x = Math.random() * (WORLD_WIDTH - 100) + 50;
	player.y = Math.random() * (WORLD_HEIGHT - 100) + 50;
	player.radius = MIN_PLAYER_RADIUS;
	player.targetX = player.x;
	player.targetY = player.y;
}

export const registry = setup({
	use: { matchmaker, gameRoom },
});
