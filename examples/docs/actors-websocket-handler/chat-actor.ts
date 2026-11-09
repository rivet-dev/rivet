import { actor } from "rivetkit";
import { db } from "rivetkit/db";

export const chatActor = actor({
    db: db({
        onMigrate: async (db) => {
            await db.execute(
                "CREATE TABLE IF NOT EXISTS messages (id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)",
            );
        },
    }),
    onWebSocket: (c, websocket) => {
        websocket.addEventListener("open", async () => {
            // Send existing messages to new connection
            const rows = (await c.db.execute(
                "SELECT text FROM messages ORDER BY id",
            )) as { text: string }[];
            websocket.send(JSON.stringify({
                type: "history",
                messages: rows.map((row) => row.text),
            }));
        });

        websocket.addEventListener("message", async (event) => {
            // Store message in SQLite so it survives restarts
            await c.db.execute("INSERT INTO messages (text) VALUES (?)", event.data as string);

            // Echo message back
            websocket.send(event.data as string);
        });
    },
    actions: {}
});
