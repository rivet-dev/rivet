import { Hono } from "hono";
import { createClient } from "rivetkit/client";
import { registry } from "./registry";

// The issuing credential stays on the backend. It is never sent to a browser.
const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT!,
	namespace: process.env.RIVET_NAMESPACE!,
	token: process.env.RIVET_ADMIN_TOKEN!,
});

// Replace this with your own session check.
async function authenticateUser(request: Request): Promise<string | null> {
	return request.headers.get("x-demo-user");
}

const app = new Hono();

app.post("/token", async (c) => {
	const userId = await authenticateUser(c.req.raw);
	if (!userId) return c.json({ error: "unauthorized" }, 401);

	// Scoped to this one actor. The default permission is gateway read.
	const profile = client.userProfile.getOrCreate(["user", userId]);
	const { token, expiresAt } = await profile.issueToken({
		subject: userId,
		expiresIn: 900,
	});

	return c.json({ actorId: await profile.resolve(), token, expiresAt }, 200, {
		"Cache-Control": "no-store",
	});
});

export default app;
