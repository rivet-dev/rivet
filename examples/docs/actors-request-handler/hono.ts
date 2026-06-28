import { actor, ActorContextOf } from "rivetkit";
import { Hono } from "hono";

// Define the actor first
const counterActor = actor({
    state: { count: 0 },
    actions: {}
});

// Build router with typed context
function buildRouter(actorCtx: ActorContextOf<typeof counterActor>) {
    const app = new Hono();

    app.get("/count", (honoCtx) => {
        return honoCtx.json({ count: actorCtx.state.count });
    });

    app.post("/increment", (honoCtx) => {
        actorCtx.state.count++;
        return honoCtx.json({ count: actorCtx.state.count });
    });

    return app;
}

// Define the full actor with onRequest
export const counterActorWithRouter = actor({
    state: { count: 0 },
    vars: { app: null as Hono | null },
    createVars: () => ({
        app: null as Hono | null
    }),
    onRequest: async (c, request) => {
        // Build router lazily
        const app = buildRouter(c as ActorContextOf<typeof counterActor>);
        return await app.fetch(request);
    },
    actions: {}
});
