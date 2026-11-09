import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { db } from "rivetkit/db";

// docs:start server
export const pollRoom = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
        CREATE TABLE IF NOT EXISTS votes (
          option TEXT PRIMARY KEY,
          count INTEGER NOT NULL
        )
      `);
		},
	}),

	// Same signature as PartyKit's onRequest: a standard Request in, a Response out
	onRequest: async (c, request) => {
		const url = new URL(request.url);

		if (request.method === "POST" && url.pathname === "/vote") {
			const { option } = (await request.json()) as { option: string };
			await c.db.execute(
				`INSERT INTO votes (option, count) VALUES (?, 1)
         ON CONFLICT(option) DO UPDATE SET count = count + 1`,
				option,
			);
			return new Response("ok");
		}

		if (request.method === "GET" && url.pathname === "/votes") {
			const votes = await c.db.execute("SELECT option, count FROM votes");
			return Response.json(votes);
		}

		return new Response("Not found", { status: 404 });
	},

	actions: {},
});
// docs:end server

export const registry = setup({ use: { pollRoom } });

// docs:start client
const client = createClient<typeof registry>("http://localhost:6420");
const poll = client.pollRoom.getOrCreate(["lunch"]);

// Replaces fetch(`${host}/parties/main/lunch/vote`, ...)
await poll.fetch("/vote", {
	method: "POST",
	body: JSON.stringify({ option: "tacos" }),
});
const response = await poll.fetch("/votes");
console.log(await response.json());
// docs:end client
