import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

// docs:start actor
// The fetch() method of a Durable Object becomes onRequest
export const api = actor({
	createVars: () => ({ hits: 0 }),
	onRequest: (c, request) => {
		const url = new URL(request.url);
		if (url.pathname === "/hello") {
			c.vars.hits += 1;
			return Response.json({ hello: c.key[0], hits: c.vars.hits });
		}
		return new Response("Not found", { status: 404 });
	},
	actions: {},
});
// docs:end actor

export const registry = setup({ use: { api } });

// docs:start client
// stub.fetch(request)
const client = createClient<typeof registry>("http://localhost:6420");
const response = await client.api.getOrCreate(["main"]).fetch("/hello");
console.log(await response.json());
// docs:end client
