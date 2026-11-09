import { Hono } from "hono";
import { type ActorContextOf, actor } from "rivetkit";
import { db } from "rivetkit/db";

export const rawFetchCounter = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS counter (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					count INTEGER NOT NULL
				);
			`);
			await db.execute(
				"INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)",
			);
		},
	}),
	createVars: () => {
		// Setup router
		return { router: createCounterRouter() };
	},
	onRequest: (c, request) => {
		return c.vars.router.fetch(request, { actor: c });
	},
	actions: {
		// ...actions...
	},
});

function createCounterRouter(): Hono<any> {
	const app = new Hono<{
		Bindings: { actor: ActorContextOf<typeof rawFetchCounter> };
	}>();

	app.get("/count", async (c) => {
		const { actor } = c.env;

		const rows = await actor.db.execute<{ count: number }>(
			"SELECT count FROM counter WHERE id = 1",
		);
		return c.json({
			count: rows[0].count,
		});
	});

	app.post("/increment", async (c) => {
		const { actor } = c.env;

		const rows = await actor.db.execute<{ count: number }>(
			"UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
		);
		return c.json({
			count: rows[0].count,
		});
	});

	return app;
}
