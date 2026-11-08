import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { z } from "zod";

const settingsSchema = z.object({
	count: z.number().default(0),
	label: z.string().default("default"), // Added in v2
});

type Settings = z.infer<typeof settingsSchema>;

const myActor = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id = 1), value TEXT NOT NULL)",
			);
		},
	}),
	// Parse stored settings on every start so fields added later get their defaults
	createVars: async (c): Promise<{ settings: Settings }> => {
		const rows = (await c.db.execute(
			"SELECT value FROM settings WHERE id = 1",
		)) as { value: string }[];
		const stored = rows[0] ? JSON.parse(rows[0].value) : {};
		return { settings: settingsSchema.parse(stored) };
	},
	actions: {
		getLabel: (c) => c.vars.settings.label,
	},
});

const registry2 = setup({ use: { myActor } });
registry2.start();
