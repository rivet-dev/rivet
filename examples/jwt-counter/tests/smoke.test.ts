import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { setTimeout } from "node:timers/promises";
import { promisify } from "node:util";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors.ts";

const issuer = process.env.DEMO_ISSUER_URL ?? "http://localhost:5173";
const issue = async () => {
	const response = await fetch(`${issuer}/api/token`, { method: "POST" });
	assert.equal(response.status, 200);
	assert.equal(response.headers.get("cache-control"), "no-store");
	return response.json();
};
const { endpoint, namespace, actorId, token } = await issue();
const issueToken = async () => (await issue()).token as string;
const details = await fetch(`${issuer}/api/counter`);
assert.equal(details.status, 200);
assert.deepEqual(await details.json(), { endpoint, namespace, actorId });
assert.equal(new URL(endpoint).username, "");
assert.equal(new URL(endpoint).password, "");
assert.equal(token.split(".").length, 3);
const claims = JSON.parse(
	Buffer.from(token.split(".")[1], "base64url").toString(),
);
assert.equal(claims.exp - claims.iat, 30, "Tokens have a 30-second lifetime");
const config = { endpoint, namespace };
const client = createClient<typeof registry>({ ...config, token });
const counter = client.counter.getForId(actorId);
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
assert.ok(
	process.env.RIVET_ENDPOINT,
	"Set the backend RIVET_ENDPOINT for smoke-test setup",
);
const serverClient = createClient<typeof registry>({
	endpoint: process.env.RIVET_ENDPOINT,
});
const otherId = await serverClient.counter
	.getOrCreate(["smoke", "other-user"])
	.resolve();
await assert.rejects(
	client.counter.getForId(otherId).getCount(),
	authError("insufficient_permissions"),
);
await assert.rejects(
	client.counter.create(["smoke", "unauthorized"]),
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
const renewedCounter = renewing.counter.getForId(actorId);
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
const recoveredCounter = recovering.counter.getForId(actorId);
await assert.rejects(recoveredCounter.increment(1), authError("invalid_token"));
assert.deepEqual(refreshRequests, [false, true]);
assert.equal(await recoveredCounter.getCount(), before + 3);
assert.equal(await recoveredCounter.increment(1), before + 4);
console.log(
	"PASS: token issuance, CLI, actor scope, metadata, expiry, renewal, and no action replay",
);

function authError(code: string) {
	return (error: unknown) =>
		error instanceof Error &&
		"group" in error &&
		error.group === "auth" &&
		"code" in error &&
		error.code === code;
}
