import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		batchOps: async (c) => {
			const encoder = new TextEncoder();

			await c.kv.batchPut([
				[encoder.encode("alpha"), encoder.encode("1")],
				[encoder.encode("beta"), encoder.encode("2")],
			]);

			const values = await c.kv.batchGet([
				encoder.encode("alpha"),
				encoder.encode("beta"),
			]);
		},
	},
});
