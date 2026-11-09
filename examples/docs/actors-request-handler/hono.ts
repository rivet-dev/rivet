import { actor, ActorContextOf } from "rivetkit";
import { db } from "rivetkit/db";
import { Hono } from "hono";

const counterDb = db({
    onMigrate: async (db) => {
        await db.execute(
            "CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY CHECK (id = 1), count INTEGER NOT NULL)",
        );
        await db.execute("INSERT OR IGNORE INTO counter (id, count) VALUES (1, 0)");
    },
});

// Define the actor first
const counterActor = actor({
    db: counterDb,
    actions: {}
});

// Build router with typed context
function buildRouter(actorCtx: ActorContextOf<typeof counterActor>) {
    const app = new Hono();

    const readCount = async () => {
        const [row] = (await actorCtx.db.execute(
            "SELECT count FROM counter WHERE id = 1",
        )) as { count: number }[];
        return row.count;
    };

    app.get("/count", async (honoCtx) => {
        return honoCtx.json({ count: await readCount() });
    });

    app.post("/increment", async (honoCtx) => {
        await actorCtx.db.execute("UPDATE counter SET count = count + 1 WHERE id = 1");
        return honoCtx.json({ count: await readCount() });
    });

    return app;
}

// Define the full actor with onRequest
export const counterActorWithRouter = actor({
    db: counterDb,
    vars: { app: null as Hono | null },
    createVars: () => ({
        app: null as Hono | null
    }),
    onRequest: async (c, request) => {
        // Build router lazily
        const app = buildRouter(c as unknown as ActorContextOf<typeof counterActor>);
        return await app.fetch(request);
    },
    actions: {}
});
