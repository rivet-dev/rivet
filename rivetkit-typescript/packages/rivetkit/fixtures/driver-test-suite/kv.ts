import { actor, type ActorContext } from "rivetkit";

export const kvActor = actor({
	actions: {
		putText: async (
			c: ActorContext<any, any, any, any, any, any>,
			key: string,
			value: string,
		) => {
			await c.kv.put(key, value);
			return true;
		},
		getText: async (
			c: ActorContext<any, any, any, any, any, any>,
			key: string,
		) => {
			return await c.kv.get(key);
		},
		listText: async (
			c: ActorContext<any, any, any, any, any, any>,
			prefix: string,
		) => {
			const results = await c.kv.list(prefix, { keyType: "text" });
			return results.map(([key, value]) => ({
				key,
				value,
			}));
		},
		listTextRange: async (
			c: ActorContext<any, any, any, any, any, any>,
			start: string,
			end: string,
			options?: {
				reverse?: boolean;
				limit?: number;
			},
		) => {
			const results = await c.kv.listRange(start, end, {
				keyType: "text",
				...options,
			});
			return results.map(([key, value]) => ({
				key,
				value,
			}));
		},
		deleteTextRange: async (
			c: ActorContext<any, any, any, any, any, any>,
			start: string,
			end: string,
		) => {
			await c.kv.deleteRange(start, end);
			return true;
		},
		roundtripArrayBuffer: async (
			c: ActorContext<any, any, any, any, any, any>,
			key: string,
			values: number[],
		) => {
			const buffer = new Uint8Array(values).buffer;
			await c.kv.put(key, buffer, { type: "arrayBuffer" });
			const result = await c.kv.get(key, { type: "arrayBuffer" });
			if (!result) {
				return null;
			}
			return Array.from(new Uint8Array(result));
		},
	},
});
