import { Hono } from "hono";
import { registry } from "./actors/index.ts";

const app = new Hono();
const handler = registry.fetchHandler({ path: "/api/rivet" });
app.all("/api/rivet/*", (c) => handler(c.req.raw));

export default app;
