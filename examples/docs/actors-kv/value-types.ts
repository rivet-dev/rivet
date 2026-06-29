import { actor } from "rivetkit";

const assets = actor({
	state: {},
	actions: {
		putAvatar: async (c, bytes: Uint8Array) => {
			await c.kv.put("avatar", bytes);
		},
		getAvatar: async (c) => {
			return await c.kv.get("avatar", { type: "binary" });
		},
		putSnapshot: async (c, data: ArrayBuffer) => {
			await c.kv.put("snapshot", data, { type: "arrayBuffer" });
		},
		getSnapshot: async (c) => {
			return await c.kv.get("snapshot", { type: "arrayBuffer" });
		},
	},
});
