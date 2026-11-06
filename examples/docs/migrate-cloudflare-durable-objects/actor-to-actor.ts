import { actor, setup } from "rivetkit";

const inventory = actor({
	createVars: () => ({ stock: new Map<string, number>([["widget", 10]]) }),
	actions: {
		reserve: (c, item: string) => {
			const left = c.vars.stock.get(item) ?? 0;
			if (left === 0) return false;
			c.vars.stock.set(item, left - 1);
			return true;
		},
	},
});

// docs:start call
// env.INVENTORY.getByName("main").reserve(item) from inside another object
const checkout = actor({
	actions: {
		buy: async (c, item: string) => {
			const client = c.client<typeof registry>();
			return await client.inventory.getOrCreate(["main"]).reserve(item);
		},
	},
});
// docs:end call

export const registry = setup({ use: { inventory, checkout } });
