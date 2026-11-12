import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { db } from "rivetkit/db";

const game = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS game (id INTEGER PRIMARY KEY CHECK (id = 1), mode TEXT NOT NULL)",
			);
		},
	}),
	onCreate: async (c, input: { mode: string }) => {
		// Store input in SQLite for later access
		await c.db.execute("INSERT INTO game (id, mode) VALUES (1, ?)", input.mode);
	},
	actions: {
		getMode: async (c) => {
			const rows = (await c.db.execute("SELECT mode FROM game WHERE id = 1")) as {
				mode: string;
			}[];
			return rows[0].mode;
		},
	},
});

const registry = setup({ use: { game } });
const client = createClient<typeof registry>("http://localhost:6420");

// Client usage
const gameHandle = client.game.getOrCreate(["game-1"], {
	createWithInput: { mode: "ranked" },
});
