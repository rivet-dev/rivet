import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";

export const counter = actor({
    db: db({
        onMigrate: async (db) => {
            await db.execute(
                "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
            );
            await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
        },
    }),
    onRequest: async (c, request) => {
        if (request.method === "POST") {
            await c.db.execute("UPDATE counter SET count = count + 1 WHERE id = 1");
        }
        const [row] = (await c.db.execute(
            "SELECT count FROM counter WHERE id = 1",
        )) as { count: number }[];
        return Response.json({ count: row.count });
    },
    actions: {}
});

export const registry = setup({ use: { counter } });
registry.start();
