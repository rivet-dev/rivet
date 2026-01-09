import { Hono } from "hono";
import { registry } from "./actors.ts";

if (!process.env.ANTHROPIC_API_KEY) {
	throw new Error("ANTHROPIC_API_KEY environment variable is required");
}

const app = new Hono();
app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
export default app;
