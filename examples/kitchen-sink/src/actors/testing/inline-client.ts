import { actor } from "rivetkit";
import type { registry } from "../../index.ts";

function isDynamicSandboxRuntime(): boolean {
	return process.cwd() === "/root";
}

async function waitForConnectionOpen(connection: {
	connStatus: string;
	onOpen(callback: () => void): () => void;
	onError(callback: (error: unknown) => void): () => void;
}) {
	if (connection.connStatus === "connected") {
		return;
	}

	await new Promise<void>((resolve, reject) => {
		const unsubscribeOpen = connection.onOpen(() => {
			unsubscribeOpen();
			unsubscribeError();
			resolve();
		});
		const unsubscribeError = connection.onError((error) => {
			unsubscribeOpen();
			unsubscribeError();
			reject(error);
		});
	});
}

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
			const client = c.client<typeof registry>();
			const count = await client.counter
				.getOrCreate(["inline-test"])
				.getCount();
			c.state.messages.push(`Got counter state: ${count}`);
			return count;
		},

		// Action that uses client with .connect() for stateful communication
		connectToCounterAndIncrement: async (c, amount: number) => {
			const client = c.client<typeof registry>();
			const handle = client.counter.getOrCreate(["inline-test-stateful"]);

			if (isDynamicSandboxRuntime()) {
				const events: number[] = [];
				const result1 = await handle.increment(amount);
				events.push(result1);
				const result2 = await handle.increment(amount * 2);
				events.push(result2);

				c.state.messages.push(
					`Connected to counter, incremented by ${amount} and ${amount * 2}, results: ${result1}, ${result2}, events: ${JSON.stringify(events)}`,
				);

				return { result1, result2, events };
			}

			await handle.getCount();
			const connection = handle.connect();
			await waitForConnectionOpen(connection);

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
