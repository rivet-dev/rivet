import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		listGreetings: async (c) => {
			const results = await c.kv.list("greeting:", { keyType: "text" });

			for (const [key, value] of results) {
				console.log(key, value);
			}
		},
	},
});
