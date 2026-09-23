import assert from "node:assert/strict";
import { createClient } from "rivetkit/client";
import { runCounter } from "../src/client.ts";
import type { registry } from "../src/counter.ts";

const endpoint = required("RIVET_ENDPOINT");
const namespace = required("RIVET_NAMESPACE");
const username = required("DEMO_USER");
const password = required("DEMO_PASSWORD");
const issuerUrl = process.env.DEMO_ISSUER_URL ?? "http://127.0.0.1:3020";
const authorization = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;

const invalidLogin = await fetch(new URL("/login", issuerUrl), {
	method: "POST",
	headers: { Authorization: "Basic invalid" },
});
assert.equal(invalidLogin.status, 401);

const initial = await runCounter(
	issuerUrl,
	endpoint,
	namespace,
	username,
	password,
);
assert.equal(initial.incremented, initial.initial + 1);
assert.equal(initial.current, initial.incremented);

// A compact reserved JWT with a bogus kid must fail closed.
// The first action is never replayed; the rejected credential only refreshes
// the *next* action, preserving at-most-once application behavior.
const bogus = `${Buffer.from(JSON.stringify({ alg: "EdDSA", kid: "bogus", typ: "rivet-auth+jwt" })).toString("base64url")}.e30.e30`;
let forcedRefreshes = 0;
const client = createClient<typeof registry>({
	endpoint,
	namespace,
	disableMetadataLookup: true,
	getToken: async ({ forceRefresh }) => {
		if (!forceRefresh) return bogus;
		forcedRefreshes++;
		const response = await fetch(new URL("/token", issuerUrl), {
			method: "POST",
			headers: { Authorization: authorization },
			cache: "no-store",
		});
		assert.equal(response.status, 200);
		return ((await response.json()) as { token: string }).token;
	},
});
const counter = client.counter.getForId(initial.actorId);
await assert.rejects(counter.increment(1), (error: unknown) => {
	return (
		error instanceof Error &&
		"group" in error &&
		error.group === "auth" &&
		"code" in error &&
		error.code === "invalid_token"
	);
});
assert.equal(forcedRefreshes, 1);
assert.equal(await counter.increment(1), initial.current + 1);
assert.equal(await counter.getCount(), initial.current + 1);
console.log(
	"JWT counter smoke passed: scoped access, rejection renewal, no action replay",
);

function required(name: string): string {
	const value = process.env[name];
	if (!value) throw new Error(`${name} must be configured`);
	return value;
}
