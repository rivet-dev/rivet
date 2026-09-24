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
	const actorId = await client.counter.getOrCreate(["demo"]).resolve();
	const response = await fetch(`${endpoint.replace(/\/$/, "")}/auth/tokens`, {
		method: "POST",
		headers: {
			Authorization: `Bearer ${engineToken}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			namespace,
			duration: 30,
			grants: [
				{
					resource: "actor_gateway",
					target: { id: actorId },
					operations: ["read"],
				},
			],
		}),
	});
	if (!response.ok) return c.json({ error: "Token issuance failed" }, 502);
	const { token } = await response.json();
	c.header("Cache-Control", "no-store");
	return c.json({ endpoint, namespace, actorId, token });
});

registry.start();
export default app;
