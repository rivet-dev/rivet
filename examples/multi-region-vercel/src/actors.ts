import { actor, setup, event } from "rivetkit";
import type { Player } from "./types.ts";

export type { Player };

interface State {
	players: Record<string, Player>;
	region: string;
}

const gameRoom = actor({
	// Create initial state with region parameter
	createState: (_c, input: { region: string }): State => ({
		players: {} as Record<string, Player>,
		region: input.region,
	}),

	// Connection state - tracks which player belongs to each connection
	connState: {
		playerId: null as string | null,
	},
	events: {
		playerJoined: event<{ playerId: string; player: Player }>(),
		playerLeft: event<{ playerId: string }>(),
		playerMoved: event<{ playerId: string; x: number; y: number }>(),
	},

	// Handle client connections
	onConnect: (c, conn) => {
		// Generate a unique player ID
		const playerId = conn.id;

		// Generate random color for player
		const colors = [
			"#FF6B6B",
			"#4ECDC4",
			"#45B7D1",
			"#FFA07A",
			"#98D8C8",
			"#F7DC6F",
			"#BB8FCE",
			"#85C1E2",
		];
		const color = colors[Math.floor(Math.random() * colors.length)];

		// Set connection state
		conn.state.playerId = playerId;

		// Add player at random position
		c.state.players[playerId] = {
			id: playerId,
			x: Math.floor(Math.random() * 900) + 50,
			y: Math.floor(Math.random() * 900) + 50,
			color,
			lastUpdate: Date.now(),
		};

		// Broadcast player joined event
		c.broadcast("playerJoined", {
			playerId,
			player: c.state.players[playerId],
		});
	},

	onDisconnect: (c, conn) => {
		const playerId = conn.state.playerId;
		if (playerId) {
			delete c.state.players[playerId];
			c.broadcast("playerLeft", { playerId });
		}
	},

	actions: {
		// Move player by delta
		move: (c, dx: number, dy: number) => {
			const playerId = c.conn.state.playerId;
			if (!playerId) return { x: 0, y: 0 };

			const player = c.state.players[playerId];
			if (!player) return { x: 0, y: 0 };

			// Update position
			player.x += dx;
			player.y += dy;

			// Keep player in bounds (0-1000)
			player.x = Math.max(0, Math.min(player.x, 1000));
			player.y = Math.max(0, Math.min(player.y, 1000));

			// Update timestamp
			player.lastUpdate = Date.now();

			// Broadcast movement
			c.broadcast("playerMoved", {
				playerId,
				x: player.x,
				y: player.y,
			});

			return { x: player.x, y: player.y };
		},

		// Get current game state
		getGameState: (c) => {
			return {
				players: c.state.players,
				region: c.state.region,
			};
		},

		// Get region info
		getRegion: (c) => {
			return c.state.region;
		},
	},
});

// Register actors for use
export const registry = setup({
	use: { gameRoom },
});
