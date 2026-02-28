import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { DEFAULT_DYNAMIC_ACTOR_SOURCE, registry } from "./actors.ts";

const app = new Hono();
const client = createClient({ encoding: "json" }) as any;

function sourceActorHandle() {
	return client.sourceCode.getOrCreate(["main"], {
		createWithInput: {
			source: DEFAULT_DYNAMIC_ACTOR_SOURCE,
		},
	});
}

function dynamicActorHandle(dynamicKey: string) {
	return client.dynamicWorkflow.getOrCreate([dynamicKey]);
}

app.get("/api/source", async (c) => {
	const sourceState = await sourceActorHandle().getSource();
	return c.json(sourceState);
});

app.post("/api/source", async (c) => {
	const body = await c.req.json();
	const source = typeof body.source === "string" ? body.source : "";
	const result = await sourceActorHandle().setSource(source);
	return c.json(result);
});

app.get("/api/dynamic/:dynamicKey/count", async (c) => {
	const dynamicKey = c.req.param("dynamicKey");
	const count = await dynamicActorHandle(dynamicKey).getCount();
	return c.json({ count });
});

app.post("/api/dynamic/:dynamicKey/increment", async (c) => {
	const dynamicKey = c.req.param("dynamicKey");
	const body = await c.req.json();
	const amount =
		typeof body.amount === "number" && Number.isFinite(body.amount)
			? body.amount
			: 1;
	const count = await dynamicActorHandle(dynamicKey).increment(amount);
	return c.json({ count });
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));

export default app;
