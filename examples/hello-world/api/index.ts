import { Hono } from "hono";
import { registry } from "./actors.ts";

const app = new Hono();
app.all("/rivet/*", (c) => registry.handler(c.req.raw));
export default app;
