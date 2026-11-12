import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export interface State {
	initialInput?: unknown;
	onCreateInput?: unknown;
}

// Test actor that can capture input during creation
export const inputActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS create_input (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					input_json TEXT NOT NULL
				);
			`);
		},
	}),

	// onCreate runs once, so the input it receives is stored for later reads.
	onCreate: async (c, input) => {
		await c.db.execute(
			"INSERT OR REPLACE INTO create_input (id, input_json) VALUES (1, ?)",
			JSON.stringify(input ?? null),
		);
	},

	actions: {
		getInputs: async (c): Promise<State> => {
			const rows = await c.db.execute<{ input_json: string }>(
				"SELECT input_json FROM create_input WHERE id = 1",
			);
			const input = rows[0] ? JSON.parse(rows[0].input_json) : undefined;
			return {
				initialInput: input ?? undefined,
				onCreateInput: input ?? undefined,
			};
		},
	},
});
