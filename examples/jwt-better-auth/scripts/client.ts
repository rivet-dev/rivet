import { createClient } from "rivetkit/client";
import { demoAccount } from "../demo-account.ts";
import type { registry } from "../src/actors.ts";

const server = process.env.DEMO_ISSUER_URL ?? "http://localhost:5173";
// Node doesn't have a browser cookie jar, so forward the login cookie explicitly.
const signedIn = await fetch(`${server}/api/auth/sign-in/email`, {
	method: "POST",
	headers: { "Content-Type": "application/json", Origin: server },
	body: JSON.stringify(demoAccount),
});
if (!signedIn.ok) throw new Error("Login failed");
const cookie = signedIn.headers
	.getSetCookie()
	.map((c) => c.split(";")[0])
	.join("; ");
const response = await fetch(`${server}/api/user`, {
	headers: { Cookie: cookie },
});
if (!response.ok) throw new Error("Could not load counter");
const { endpoint, namespace, actorId } = await response.json();

const client = createClient<typeof registry>({
	endpoint,
	namespace,
	getToken: async () => {
		const response = await fetch(`${server}/api/token`, {
			method: "POST",
			headers: { Cookie: cookie },
		});
		if (!response.ok) throw new Error("Could not get actor token");
		return (await response.json()).token;
	},
});
const counter = client.user.getForId(actorId);
const before = await counter.getCount();
const after = await counter.increment(1);
console.log(`Counter: ${before} → ${after}`);
await fetch(`${server}/api/auth/sign-out`, {
	method: "POST",
	headers: {
		Cookie: cookie,
		Origin: server,
		"Content-Type": "application/json",
	},
	body: "{}",
});
