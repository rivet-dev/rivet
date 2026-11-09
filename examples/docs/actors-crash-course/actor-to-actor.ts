import { actor, setup } from "rivetkit";

const inventory = actor({
	vars: { stock: 100 },
	actions: {
		reserve: (c, amount: number) => {
			c.vars.stock -= amount;
		},
	},
});

const order = actor({
	actions: {
		process: async (c) => {
			const client = c.client<typeof registry>();
			await client.inventory.getOrCreate(["main"]).reserve(1);
		},
	},
});

const registry = setup({ use: { inventory, order } });
