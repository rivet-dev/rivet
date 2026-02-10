import { actor } from "rivetkit";
import type { registry } from "../../actors.ts";

export const inlineClientActor = actor({
	state: { messages: [] as string[] },
	actions: {
		// Action that uses client to call another actor (stateless)
		callCounterIncrement: async (c, amount: number) => {
			const client = c.client<typeof registry>();
			const result = await client.counter
				.getOrCreate(["inline-test"])
				.increment(amount);
			c.state.messages.push(
				`Called counter.increment(${amount}), result: ${result}`,
			);
			return result;
		},

		// Action that uses client to get counter state (stateless)
		getCounterState: async (c) => {
			const client = c.client();
			const count = await client.counter
				.getOrCreate(["inline-test"])
				.getCount();
			c.state.messages.push(`Got counter state: ${count}`);
			return count;
		},

		// Action that uses client with .connect() for stateful communication
		connectToCounterAndIncrement: async (c, amount: number) => {
			const client = c.client();
			const handle = client.counter.getOrCreate(["inline-test-stateful"]);
			const connection = handle.connect();

			// Set up event listener
			const events: number[] = [];
			connection.on("newCount", (count: number) => {
				events.push(count);
			});

			// Perform increments
			const result1 = await connection.increment(amount);
			const result2 = await connection.increment(amount * 2);

			await connection.dispose();

			c.state.messages.push(
				`Connected to counter, incremented by ${amount} and ${amount * 2}, results: ${result1}, ${result2}, events: ${JSON.stringify(events)}`,
			);

			return { result1, result2, events };
		},

		// Get all messages from this actor's state
		getMessages: (c) => {
			return c.state.messages;
		},

		// Clear messages
		clearMessages: (c) => {
			c.state.messages = [];
		},
	},
});
