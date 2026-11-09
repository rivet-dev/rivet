import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const counterActor = actor({
    db: db({
        onMigrate: async (db) => {
            await db.execute(
                "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
            );
            await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
        },
    }),
    // WinterTC compliant - accepts standard Request and returns standard Response
    onRequest: async (c, request) => {
        const url = new URL(request.url);

        if (request.method === "GET" && url.pathname === "/count") {
            const [{ count }] = await c.db.execute<{ count: number }>(
                "SELECT count FROM counter WHERE id = 1",
            );
            return Response.json({ count });
        }

        if (request.method === "POST" && url.pathname === "/increment") {
            const [{ count }] = await c.db.execute<{ count: number }>(
                "UPDATE counter SET count = count + 1 WHERE id = 1 RETURNING count",
            );
            return Response.json({ count });
        }

        return new Response("Not Found", { status: 404 });
    },
    actions: {}
});
