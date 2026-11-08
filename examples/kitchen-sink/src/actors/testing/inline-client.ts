import { actor } from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../../index.ts";

// The dynamic-sandbox WebSocket connect path is failing intermittently and
// silently swapping to HTTP-only would skip event-bridge validation entirely.
// Re-enable WebSocket coverage on every runtime; if the dynamic-sandbox
// regression returns, root-cause it instead of restoring this gate.
function isDynamicSandboxRuntime(): boolean {
	return false;
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
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					text TEXT NOT NULL
				);
			`);
		},
	}),
	actions: {
		// Action that uses client to call another actor (stateless)
		callCounterIncrement: async (c, amount: number) => {
			const client = c.client<typeof registry>();
			const result = await client.counter
				.getOrCreate(["inline-test"])
				.increment(amount);
			await c.db.execute(
				"INSERT INTO messages (text) VALUES (?)",
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
			await c.db.execute(
				"INSERT INTO messages (text) VALUES (?)",
				`Got counter state: ${count}`,
			);
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

				await c.db.execute(
					"INSERT INTO messages (text) VALUES (?)",
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

			await c.db.execute(
				"INSERT INTO messages (text) VALUES (?)",
				`Connected to counter, incremented by ${amount} and ${amount * 2}, results: ${result1}, ${result2}, events: ${JSON.stringify(events)}`,
			);

			return { result1, result2, events };
		},

		// Get all messages recorded by this actor
		getMessages: async (c) => {
			const rows = await c.db.execute<{ text: string }>(
				"SELECT text FROM messages ORDER BY id",
			);
			return rows.map((row) => row.text);
		},

		// Clear messages
		clearMessages: async (c) => {
			await c.db.execute("DELETE FROM messages");
		},
	},
});
