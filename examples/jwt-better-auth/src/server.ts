import { Hono, type Context } from "hono";
import { HTTPException } from "hono/http-exception";
import { createClient } from "rivetkit/client";
import { registry } from "./actors.ts";
import { auth } from "./auth.ts";

// Use the same backend credentials for hosting actors and issuing tokens.
const { endpoint, namespace, token: engineToken } = registry.parseConfig();
if (!endpoint || !engineToken) {
	throw new Error("Set RIVET_ENDPOINT (including credentials)");
}
const client = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT,
});

const app = new Hono();
app.on(["GET", "POST"], "/api/auth/*", (c) => auth.handler(c.req.raw));

app.get("/api/user", async (c) => {
	const { actorId } = await getUserActor(c);
	c.header("Cache-Control", "no-store");
	return c.json({ endpoint, namespace, actorId });
});

app.post("/api/token", async (c) => {
	// Derive actor access from the session, never from a browser-supplied user or actor ID.
	const { user, userId } = await getUserActor(c);
	c.header("Cache-Control", "no-store");

	// Ask Engine for a short-lived token that can access only this user's counter.
	try {
		const { token } = await user.issueToken({
			subject: userId,
			expiresIn: 30,
		});
		return c.json({ token });
	} catch (error) {
		console.error("Token issuance failed", error);
		return c.json({ error: "Token issuance failed" }, 502);
	}
});

registry.start();
export default app;

async function getUserActor(c: Context) {
	const session = await auth.api.getSession({ headers: c.req.raw.headers });
	if (!session) throw new HTTPException(401, { message: "Log in first" });
	const userId = session.user.id;
	const user = client.user.getOrCreate(["user", userId]);
	const actorId = await user.resolve();
	return { userId, actorId, user };
}
