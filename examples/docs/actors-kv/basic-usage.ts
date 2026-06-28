import { actor } from "rivetkit";

const greetings = actor({
	state: {},
	actions: {
		setGreeting: async (c, userId: string, message: string) => {
			await c.kv.put(`greeting:${userId}`, message);
		},
		getGreeting: async (c, userId: string) => {
			return await c.kv.get(`greeting:${userId}`);
		},
	},
});
