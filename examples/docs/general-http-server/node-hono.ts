import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { actor, setup } from "rivetkit";

const counter = actor({ state: { count: 0 }, actions: {} });
const registry = setup({ use: { counter } });

registry.start();

const app = new Hono();
app.get("/health", () => registry.routes.health());
app.get("/metrics", (c) => registry.routes.prometheusMetrics(c.req.raw));

serve({ fetch: app.fetch, port: 3000 });
