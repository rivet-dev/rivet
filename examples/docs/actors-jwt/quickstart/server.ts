import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

const endpoint = process.env.RIVET_ENDPOINT!;
const namespace = process.env.RIVET_NAMESPACE!;

// The admin token stays on the backend. It is never sent to a browser.
const adminToken = process.env.RIVET_ADMIN_TOKEN!;

const adminClient = createClient<typeof registry>({
	endpoint,
	namespace,
	token: adminToken,
});

// Replace this with your own session check.
async function authenticateUser(request: Request): Promise<string | null> {
	const userId = request.headers.get("x-demo-user");
	return userId ?? null;
}

const app = new Hono();

app.post("/token", async (c) => {
	const userId = await authenticateUser(c.req.raw);
	if (!userId) return c.json({ error: "unauthorized" }, 401);

	// Resolve the one actor this user is allowed to reach.
	const actorId = await adminClient.userProfile
		.getOrCreate(["user", userId])
		.resolve();

	const response = await fetch(new URL("/auth/tokens", endpoint), {
		method: "POST",
		headers: {
			Authorization: `Bearer ${adminToken}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			namespace,
			subject: userId,
			duration: 900,
			grants: [
				{
					resource: "actor_gateway",
					target: { id: actorId },
					operations: ["read"],
				},
			],
		}),
	});

	if (!response.ok) return c.json({ error: "issuer_unavailable" }, 503);

	const issued = (await response.json()) as { token: string };
	return c.json({ actorId, token: issued.token }, 200, {
		"Cache-Control": "no-store",
	});
});

export default app;
