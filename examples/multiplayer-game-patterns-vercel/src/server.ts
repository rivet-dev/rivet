import { Hono } from "hono";
import { registry } from "./actors/index.ts";

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
