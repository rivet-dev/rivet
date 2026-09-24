import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors.ts";

const server = process.env.DEMO_ISSUER_URL ?? "http://localhost:5173";
const response = await fetch(`${server}/api/counter`);
if (!response.ok) throw new Error("Could not load the counter");
const { endpoint, namespace, actorId } = await response.json();
const client = createClient<typeof registry>({
	endpoint,
	namespace,
	getToken: async () => {
		const response = await fetch(`${server}/api/token`, { method: "POST" });
		if (!response.ok) throw new Error("Could not get a token");
		return (await response.json()).token;
	},
});
const counter = client.counter.getForId(actorId);
const before = await counter.getCount();
const after = await counter.increment(1);
console.log(`Counter: ${before} → ${after}`);
