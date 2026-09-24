import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { setTimeout } from "node:timers/promises";
import { promisify } from "node:util";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors.ts";
import { demoAccount } from "../demo-account.ts";

const issuer = process.env.DEMO_ISSUER_URL ?? "http://localhost:5173";
for (const [path, method] of [
	["/api/user", "GET"],
	["/api/token", "POST"],
]) {
	assert.equal((await fetch(`${issuer}${path}`, { method })).status, 401);
}
const badPassword = await fetch(`${issuer}/api/auth/sign-in/email`, {
	method: "POST",
	headers: { "Content-Type": "application/json", Origin: issuer },
	body: JSON.stringify({ ...demoAccount, password: "wrong-password" }),
});
assert.equal(badPassword.status, 401);
const signedIn = await fetch(`${issuer}/api/auth/sign-in/email`, {
	method: "POST",
	headers: { "Content-Type": "application/json", Origin: issuer },
	body: JSON.stringify(demoAccount),
});
assert.equal(signedIn.status, 200);
const cookie = signedIn.headers
	.getSetCookie()
	.map((c) => c.split(";")[0])
	.join("; ");
assert.ok(cookie);
assert.ok(signedIn.headers.getSetCookie().some((c) => /HttpOnly/i.test(c)));
const counterResponse = await fetch(`${issuer}/api/user`, {
	headers: { Cookie: cookie },
});
assert.equal(counterResponse.status, 200);
const { endpoint, namespace, actorId } = await counterResponse.json();
const issueToken = async () => {
	const response = await fetch(`${issuer}/api/token`, {
		method: "POST",
		headers: { Cookie: cookie },
	});
	assert.equal(response.status, 200);
	assert.equal(response.headers.get("cache-control"), "no-store");
	return ((await response.json()) as { token: string }).token;
};
const token = await issueToken();
assert.equal(new URL(endpoint).username, "");
assert.equal(new URL(endpoint).password, "");
assert.equal(token.split(".").length, 3);
const config = { endpoint, namespace };
const client = createClient<typeof registry>({ ...config, token });
const counter = client.user.getForId(actorId);
const before = await counter.getCount();
assert.equal(await counter.increment(1), before + 1);
assert.equal(await counter.getCount(), before + 1);

// Run the actual user-facing script without the server’s Engine endpoint or credentials.
const clientEnv = { ...process.env };
for (const name of [
	"RIVET_ENDPOINT",
	"RIVET_ENGINE",
	"RIVET_TOKEN",
	"RIVET_NAMESPACE",
	"RIVET_ADMIN_TOKEN",
])
	delete clientEnv[name];
const { stdout } = await promisify(execFile)(
	process.execPath,
	["--import", "tsx", "scripts/client.ts"],
	{ env: clientEnv, timeout: 30_000 },
);
assert.match(stdout, new RegExp(`Counter: ${before + 1} → ${before + 2}`));

// The scoped token cannot reach someone else's actor or create actors.
const signedUp = await fetch(`${issuer}/api/auth/sign-up/email`, {
	method: "POST",
	headers: { "Content-Type": "application/json", Origin: issuer },
	body: JSON.stringify({
		name: "Smoke user",
		email: `smoke-${randomUUID()}@example.com`,
		password: "test-password-123",
	}),
});
assert.equal(signedUp.status, 200);
const otherCookie = signedUp.headers
	.getSetCookie()
	.map((c) => c.split(";")[0])
	.join("; ");
assert.ok(otherCookie, "Signup establishes a login session");
const otherResponse = await fetch(`${issuer}/api/user`, {
	headers: { Cookie: otherCookie },
});
assert.equal(otherResponse.status, 200);
const { actorId: otherId } = await otherResponse.json();
assert.notEqual(otherId, actorId, "Each user gets a separate user actor");
const otherTokenResponse = await fetch(`${issuer}/api/token`, {
	method: "POST",
	headers: { Cookie: otherCookie },
});
assert.equal(otherTokenResponse.status, 200);
const otherClient = createClient<typeof registry>({
	...config,
	token: (await otherTokenResponse.json()).token,
});
assert.equal(await otherClient.user.getForId(otherId).getCount(), 0);
await assert.rejects(
	otherClient.user.getForId(actorId).getCount(),
	authError("insufficient_permissions"),
);
await assert.rejects(
	client.user.getForId(otherId).getCount(),
	authError("insufficient_permissions"),
);
await assert.rejects(
	client.user.create(["smoke", "unauthorized"]),
	authError("insufficient_permissions"),
);

// Long-lived clients can ask the backend for another token through getToken.
let issued = 0;
const renewing = createClient<typeof registry>({
	...config,
	getToken: async () => {
		issued++;
		return await issueToken();
	},
});
const renewedCounter = renewing.user.getForId(actorId);
assert.equal(await renewedCounter.getCount(), before + 2);
assert.equal(await renewedCounter.getCount(), before + 2);
assert.equal(issued, 1, "RivetKit caches the token between calls");

// RivetKit renews after the 30-second token lifetime.
await setTimeout(36_000);
assert.equal(await renewedCounter.increment(1), before + 3);
assert.equal(issued, 2);

// Engine allows 30 seconds of clock skew after exp; wait beyond that too.
await setTimeout(30_000);
await assert.rejects(counter.increment(1), authError("token_expired"));

// Rejected credentials trigger renewal, but the failed action is never replayed.
const bogus = `${Buffer.from(JSON.stringify({ alg: "EdDSA", kid: "bogus", typ: "rivet-auth+jwt" })).toString("base64url")}.e30.e30`;
const refreshRequests: boolean[] = [];
const recovering = createClient<typeof registry>({
	...config,
	getToken: async ({ forceRefresh }) => {
		refreshRequests.push(forceRefresh);
		return forceRefresh ? await issueToken() : bogus;
	},
});
const recoveredCounter = recovering.user.getForId(actorId);
await assert.rejects(recoveredCounter.increment(1), authError("invalid_token"));
assert.deepEqual(refreshRequests, [false, true]);
assert.equal(await recoveredCounter.getCount(), before + 3);
assert.equal(await recoveredCounter.increment(1), before + 4);
// Signing out revokes the session, including a saved copy of its cookie.
const signedOut = await fetch(`${issuer}/api/auth/sign-out`, {
	method: "POST",
	headers: {
		Cookie: cookie,
		Origin: issuer,
		"Content-Type": "application/json",
	},
	body: "{}",
});
assert.equal(signedOut.status, 200);
for (const [path, method] of [
	["/api/user", "GET"],
	["/api/token", "POST"],
]) {
	assert.equal(
		(
			await fetch(`${issuer}${path}`, {
				method,
				headers: { Cookie: cookie },
			})
		).status,
		401,
	);
}
console.log(
	"PASS: signup, login/logout, per-user actors, CLI, metadata, expiry, session-based renewal, and no action replay",
);

function authError(code: string) {
	return (error: unknown) =>
		error instanceof Error &&
		"group" in error &&
		error.group === "auth" &&
		"code" in error &&
		error.code === code;
}
