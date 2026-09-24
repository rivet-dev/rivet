import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./actors.ts";

const { endpoint, namespace, token: engineToken } = registry.parseConfig();
if (!endpoint || !engineToken)
	throw new Error("Set RIVET_ENDPOINT (including credentials)");
const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT,
});

const app = new Hono();
app.get("/api/counter", async (c) => {
	const actorId = await client.counter.getOrCreate(["demo"]).resolve();
	c.header("Cache-Control", "no-store");
	return c.json({ endpoint, namespace, actorId });
});

app.post("/api/token", async (c) => {
	// This public demo shares one counter. See jwt-better-auth for per-user access.
	const counter = client.counter.getOrCreate(["demo"]);
	const actorId = await counter.resolve();
	let token: string;
	try {
		({ token } = await counter.issueToken({ expiresIn: 30 }));
	} catch {
		return c.json({ error: "Token issuance failed" }, 502);
	}
	c.header("Cache-Control", "no-store");
	return c.json({ endpoint, namespace, actorId, token });
});

registry.start();
export default app;
