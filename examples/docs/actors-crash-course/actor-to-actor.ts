import { actor, setup } from "rivetkit";

const inventory = actor({
	state: { stock: 100 },
	actions: {
		reserve: (c, amount: number) => {
			c.state.stock -= amount;
		},
	},
});

const order = actor({
	state: {},
	actions: {
		process: async (c) => {
			const client = c.client<typeof registry>();
			await client.inventory.getOrCreate(["main"]).reserve(1);
		},
	},
});

const registry = setup({ use: { inventory, order } });
