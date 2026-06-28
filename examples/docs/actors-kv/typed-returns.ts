import { actor } from "rivetkit";

const example = actor({
	state: {},
	actions: {
		demo: async (c) => {
			const textValue = await c.kv.get("greeting");
			//    ^? string | null

			const bytes = await c.kv.get("avatar", { type: "binary" });
			//    ^? Uint8Array | null
		},
	},
});
