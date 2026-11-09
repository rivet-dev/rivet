import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db/drizzle";
import migrations from "../drizzle/migrations.js";
import * as schema from "./db/schema.ts";

const { usersTable } = schema;

export const userDirectory = actor({
	// Each actor has its own SQLite database, queried with Drizzle: https://rivet.dev/docs/actors/sqlite
	db: db({ schema, migrations }),
	actions: {
		addUser: async (c, name: string, age: number, email: string) => {
			const rows = await c.db
				.insert(usersTable)
				.values({ name, age, email })
				.returning();
			return rows[0];
		},
		listUsers: async (c) => {
			return await c.db.select().from(usersTable).orderBy(usersTable.id);
		},
	},
});

// Register actors and start the server: https://rivet.dev/docs/setup
export const registry = setup({
	use: { userDirectory },
});

registry.start();
