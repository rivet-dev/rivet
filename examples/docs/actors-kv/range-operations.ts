import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		pruneAndScan: async (c) => {
			const active = await c.kv.listRange("job:", "joc:", {
				keyType: "text",
			});

			const encoder = new TextEncoder();
			await c.kv.deleteRange(
				encoder.encode("job:old:"),
				encoder.encode("job:old;"),
			);

			return active.map(([key, value]) => ({ key, value }));
		},
	},
});
