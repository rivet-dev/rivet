import { Hono } from "hono";
import { actor, setup } from "rivetkit";

const myActor = actor({ state: {}, actions: {} });
const registry = setup({ use: { myActor } });

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
