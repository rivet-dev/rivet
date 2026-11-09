import { actor, UserError } from "rivetkit";
import { db } from "rivetkit/db";

interface ConnParams {
	userId?: string;
}

const userProfile = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS profile (id INTEGER PRIMARY KEY CHECK (id = 1), owner_id TEXT NOT NULL, is_private INTEGER NOT NULL)",
			);
		},
	}),

	// Record the owner once, when the profile is first created
	onCreate: async (c, input: { ownerId: string }) => {
		await c.db.execute(
			"INSERT INTO profile (id, owner_id, is_private) VALUES (1, ?, 1)",
			input.ownerId,
		);
	},

	// Keep the profile settings in memory for fast permission checks
	createVars: async (c): Promise<{ ownerId: string; isPrivate: boolean }> => {
		const rows = (await c.db.execute(
			"SELECT owner_id, is_private FROM profile WHERE id = 1",
		)) as { owner_id: string; is_private: number }[];
		return { ownerId: rows[0].owner_id, isPrivate: rows[0].is_private === 1 };
	},

	onBeforeConnect: (c, params: ConnParams) => {
		// Use the actor's own data to check access permissions
		if (c.vars.isPrivate && params.userId !== c.vars.ownerId) {
			throw new UserError("Access denied to private profile", {
				code: "forbidden",
			});
		}
	},

	actions: {
		getProfile: (c) => ({ ownerId: c.vars.ownerId }),
	},
});
