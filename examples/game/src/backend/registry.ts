import { actor, setup } from "rivetkit";
import type { GameVars, Input, Player } from "./types";

export type { Player };

const gameRoom = actor({
	// Persistent state that survives restarts
	state: {
		players: {} as Record<string, Player>,
		mapSize: 800,
	},

	createVars: (): GameVars => ({}),

	onWake: (c) => {
		// Set up game update loop
		c.vars.gameLoopInterval = setInterval(() => {
			const playerList: Player[] = [];
			let hasPlayers = false;

			for (const id in c.state.players) {
				const player = c.state.players[id];
				const speed = 5;

				// Update position based on input
				player.position.x += player.input.x * speed;
				player.position.y += player.input.y * speed;

				// Keep player in bounds
				player.position.x = Math.max(
					10,
					Math.min(player.position.x, c.state.mapSize - 10),
				);
				player.position.y = Math.max(
					10,
					Math.min(player.position.y, c.state.mapSize - 10),
				);

				// Add to list for broadcast
				playerList.push(player);
				hasPlayers = true;
			}

			// Only broadcast if there are players
			if (hasPlayers) {
				// Send events to all connected clients
				c.broadcast("worldUpdate", { playerList });
			}
		}, 50);
	},

	onSleep: (c) => {
		if (c.vars.gameLoopInterval) {
			clearInterval(c.vars.gameLoopInterval);
		}
	},

	// Handle client connections
	onConnect: (c, conn) => {
		const id = conn.id;
		// State changes are automatically persisted
		c.state.players[id] = {
			id,
			position: {
				x: Math.floor(Math.random() * (c.state.mapSize - 100)) + 50,
				y: Math.floor(Math.random() * (c.state.mapSize - 100)) + 50,
			},
			input: { x: 0, y: 0 },
		};

		// Send initial world state to new player
		const playerList = Object.values(c.state.players);
		conn.send("worldUpdate", { playerList });
	},

	onDisconnect: (c, conn) => {
		delete c.state.players[conn.id];
	},

	actions: {
		// Callable functions from clients
		setInput: (c, input: Input) => {
			const player = c.state.players[c.conn.id];
			if (player) {
				player.input = input;
			}
		},

		getPlayerCount: (c) => {
			return Object.keys(c.state.players).length;
		},
	},
});

// Register actors for use
export const registry = setup({
	use: { gameRoom },
});
