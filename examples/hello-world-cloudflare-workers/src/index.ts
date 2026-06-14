import { createHandler, setup } from "@rivetkit/cloudflare-workers";
import { actor } from "rivetkit";

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

// Exported so `src/client.ts` can type the client with `typeof registry`.
export const registry = setup({ use: { counter } });

export default createHandler(registry);
