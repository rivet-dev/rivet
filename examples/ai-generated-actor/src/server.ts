import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./actors.ts";

const app = new Hono();
const client = createClient<typeof registry>({ encoding: "json" });

// Call an arbitrary action on the dynamic actor by name.
app.post("/api/dynamic/:key/:version/action", async (c) => {
	const key = c.req.param("key");
	const version = c.req.param("version");
	const body = await c.req.json();
	const actionName = body.name;
	const args = Array.isArray(body.args) ? body.args : [];

	if (!actionName || typeof actionName !== "string") {
		return c.json({ error: "Missing action name" }, 400);
	}

	try {
		const handle = client.dynamicRunner.getOrCreate([key, version]);
		const result = await (handle as any)[actionName](...args);
		return c.json({ result });
	} catch (error) {
		const message =
			error instanceof Error ? error.message : "Action call failed";
		return c.json({ error: message }, 500);
	}
});

// Send a message to a queue on the dynamic actor.
app.post("/api/dynamic/:key/:version/queue", async (c) => {
	const key = c.req.param("key");
	const version = c.req.param("version");
	const body = await c.req.json();
	const queueName = body.name;
	const message = body.message;

	if (!queueName || typeof queueName !== "string") {
		return c.json({ error: "Missing queue name" }, 400);
	}

	try {
		const handle = client.dynamicRunner.getOrCreate([key, version]);
		await (handle as any).send(queueName, message);
		return c.json({ ok: true });
	} catch (error) {
		const msg =
			error instanceof Error ? error.message : "Queue send failed";
		return c.json({ error: msg }, 500);
	}
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
