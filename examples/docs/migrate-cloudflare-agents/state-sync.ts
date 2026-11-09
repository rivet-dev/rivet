import { actor, event, setup, UserError } from "rivetkit";
import { db } from "rivetkit/db";

interface GameState {
	score: number;
	players: string[];
}

export const game = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS score (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          value INTEGER NOT NULL
        )
      `);
			await db.execute(
				"INSERT OR IGNORE INTO score (id, value) VALUES (1, 0)",
			);
			await db.execute(
				"CREATE TABLE IF NOT EXISTS players (name TEXT PRIMARY KEY)",
			);
		},
	}),

	events: {
		stateChanged: event<GameState>(),
	},

	// Load durable data into memory on every start, like initialState
	createVars: async (c): Promise<{ game: GameState }> => {
		const score = await c.db.execute<{ value: number }>(
			"SELECT value FROM score WHERE id = 1",
		);
		const players = await c.db.execute<{ name: string }>(
			"SELECT name FROM players ORDER BY name",
		);
		return {
			game: {
				score: score[0].value,
				players: players.map((p) => p.name),
			},
		};
	},

	actions: {
		// Clients load the state once, then follow stateChanged events
		getState: (c) => c.vars.game,

		// Clients write through actions, so validate here like validateStateChange
		addScore: async (c, delta: number) => {
			const next = c.vars.game.score + delta;
			if (!Number.isInteger(delta) || next < 0) {
				throw new UserError("score must stay a non-negative integer");
			}
			// Change memory before the await and write the delta, so concurrent
			// calls don't lose updates
			c.vars.game.score = next;
			await c.db.execute(
				"UPDATE score SET value = value + ? WHERE id = 1",
				delta,
			);
			c.broadcast("stateChanged", c.vars.game);
			return next;
		},

		addPlayer: async (c, name: string) => {
			if (c.vars.game.players.includes(name)) return;
			c.vars.game.players.push(name);
			await c.db.execute(
				"INSERT OR IGNORE INTO players (name) VALUES (?)",
				name,
			);
			c.broadcast("stateChanged", c.vars.game);
		},
	},
});

export const registry = setup({ use: { game } });
registry.start();
