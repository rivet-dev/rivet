import { createHandler, setup } from "@rivetkit/cloudflare-workers";
import { actor } from "rivetkit";
import { createClient } from "rivetkit/client";

const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});

export const registry = setup({ use: { counter } });

interface Env {
	RIVET_ENDPOINT: string;
}

// Mount a hand-rolled router alongside Rivet. The Rivet manager API stays on
// `/api/rivet`; every other route is handled by this `fetch`.
export default createHandler(registry, {
	fetch: async (request: Request, env: Env) => {
		const url = new URL(request.url);

		if (url.pathname === "/") {
			return new Response("Hello from a raw Rivet Worker router!");
		}

		const increment = url.pathname.match(/^\/increment\/(.+)$/);
		if (request.method === "POST" && increment) {
			const client = createClient<typeof registry>({
				endpoint: env.RIVET_ENDPOINT,
			});
			const count = await client.counter
				.getOrCreate(increment[1])
				.increment(1);
			return Response.json({ count });
		}

		return new Response("Not found", { status: 404 });
	},
});
