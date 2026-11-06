import { createHmac, timingSafeEqual } from "node:crypto";
import { serve } from "@hono/node-server";
import { Hono } from "hono";
import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { db } from "rivetkit/db";

const WEBHOOK_SECRET = process.env.WEBHOOK_SECRET ?? "";

function verifySignature(body: string, signature: string | null): boolean {
	if (!signature) return false;
	const expected = `sha256=${createHmac("sha256", WEBHOOK_SECRET).update(body).digest("hex")}`;
	return (
		expected.length === signature.length &&
		timingSafeEqual(Buffer.from(expected), Buffer.from(signature))
	);
}

// docs:start actor
export const repoAgent = actor({
	db: db({
		onMigrate: async (db) => {
			await db.execute(
				"CREATE TABLE IF NOT EXISTS deliveries (id TEXT PRIMARY KEY, payload TEXT NOT NULL)",
			);
		},
	}),

	onRequest: async (c, request) => {
		const body = await request.text();
		if (
			!verifySignature(body, request.headers.get("x-hub-signature-256"))
		) {
			return new Response("Invalid signature", { status: 401 });
		}

		// Providers retry deliveries, so store each delivery ID once
		const deliveryId =
			request.headers.get("x-github-delivery") ?? crypto.randomUUID();
		await c.db.execute(
			"INSERT OR IGNORE INTO deliveries (id, payload) VALUES (?, ?)",
			deliveryId,
			body,
		);
		return new Response("OK");
	},

	actions: {},
});
// docs:end actor

export const registry = setup({ use: { repoAgent } });
registry.start();

// docs:start router
const client = createClient<typeof registry>("http://localhost:6420");
const app = new Hono();

// Like getAgentByName(): the URL picks the actor key
app.post("/webhooks/:repo", async (c) => {
	const agent = client.repoAgent.getOrCreate([c.req.param("repo")]);
	return agent.fetch("/", {
		method: "POST",
		headers: c.req.raw.headers,
		body: await c.req.text(),
	});
});

serve({ fetch: app.fetch, port: 3000 });
// docs:end router
