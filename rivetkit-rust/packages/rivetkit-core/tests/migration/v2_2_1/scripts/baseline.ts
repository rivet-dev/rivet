import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";
import { db } from "rivetkit/db";

const ACTOR_NAME = "actor-v2-2-1-baseline";
const TOKEN = process.env.RIVET_TOKEN ?? "dev";
const NAMESPACE = process.env.RIVET_NAMESPACE ?? "default";
const ENDPOINT = requireEnv("RIVET_ENDPOINT");

const baselineActor = actor({
	state: {
		source: "v2.2.1",
		counter: 0,
		migrated: false,
		scheduledCount: 0,
	},
	db: db({
		onMigrate: async (sql) => {
			await sql.execute(
				"CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY, note TEXT NOT NULL)",
			);
			await sql.execute(
				"CREATE TABLE IF NOT EXISTS migrations_log (id INTEGER PRIMARY KEY, note TEXT NOT NULL)",
			);
			await sql.execute(
				"INSERT INTO migrations_log(note) VALUES (?)",
				"migrated-from-v2.2.1",
			);
		},
	}),
	actions: {
		seed: async (c) => {
			c.state.counter = 42;
			c.state.migrated = true;

			await c.kv.put("snapshot-key", "snapshot-value");
			await c.kv.put("binary-key", new Uint8Array([1, 3, 3, 7]), {
				type: "binary",
			});
			await c.kv.putBatch([
				["manual-batch-a", "manual-a"],
				["manual-batch-b", "manual-b"],
			]);
			await c.kv.put("overwrite-key", "before");
			await c.kv.put("overwrite-key", "after");
			await c.kv.put("delete-key", "delete-me");
			await c.kv.delete("delete-key");

			await c.db.execute(
				"INSERT INTO items(note) VALUES (?)",
				"sqlite-from-v2.2.1",
			);
			await c.schedule.at(Date.now() + 10_000, "scheduled", {
				source: "v2.2.1",
				ok: true,
			});
			await c.saveState({ immediate: true });

			return { actorId: c.actorId };
		},
		scheduled: async (c, payload: unknown) => {
			c.state.scheduledCount++;
			await c.kv.put("scheduled-payload", JSON.stringify(payload));
			await c.db.execute("INSERT INTO items(note) VALUES (?)", "scheduled-ran");
			await c.saveState({ immediate: true });
		},
		verify: async (c) => {
			const binary = await c.kv.get("binary-key", { type: "binary" });
			const list = await c.kv.list("manual-batch-");
			const rows = await c.db.execute<{ note: string }>(
				"SELECT note FROM items ORDER BY id",
			);
			const migrations = await c.db.execute<{ note: string }>(
				"SELECT note FROM migrations_log ORDER BY id",
			);

			return {
				state: c.state,
				snapshot: await c.kv.get("snapshot-key"),
				binary: binary ? Array.from(binary) : null,
				manualBatch: list,
				overwrite: await c.kv.get("overwrite-key"),
				deleted: await c.kv.get("delete-key"),
				rows: rows.map((row) => row.note),
				migrations: migrations.map((row) => row.note),
			};
		},
	},
});

const registry = setup({
	use: { [ACTOR_NAME]: baselineActor },
	endpoint: ENDPOINT,
	token: TOKEN,
	namespace: NAMESPACE,
	runner: { runnerName: "default" },
	logging: { level: "info" },
});
registry.startRunner();

const client = createClient<typeof registry>({
	endpoint: ENDPOINT,
	token: TOKEN,
	namespace: NAMESPACE,
	runnerName: "default",
	encoding: "json",
	devtools: false,
});

const handle = await retry(() => client[ACTOR_NAME].getOrCreate());
const seeded = await retry(() => handle.seed());
const verified = await retry(() => handle.verify());

assertEqual(verified, {
	state: {
		source: "v2.2.1",
		counter: 42,
		migrated: true,
		scheduledCount: 0,
	},
	snapshot: "snapshot-value",
	binary: [1, 3, 3, 7],
	manualBatch: [
		["manual-batch-a", "manual-a"],
		["manual-batch-b", "manual-b"],
	],
	overwrite: "after",
	deleted: null,
	rows: ["sqlite-from-v2.2.1"],
	migrations: ["migrated-from-v2.2.1"],
});

console.log(JSON.stringify({ seeded, verified }, null, 2));
await client.dispose();
await new Promise((resolve) => setTimeout(resolve, 1000));
process.exit(0);

function requireEnv(name: string): string {
	const value = process.env[name];
	if (!value) {
		throw new Error(`${name} is required`);
	}
	return value;
}

async function retry<T>(fn: () => Promise<T>): Promise<T> {
	let lastError: unknown;
	for (let i = 0; i < 120; i++) {
		try {
			return await fn();
		} catch (error) {
			lastError = error;
			await new Promise((resolve) => setTimeout(resolve, 500));
		}
	}
	throw lastError;
}

function assertEqual(actual: unknown, expected: unknown) {
	const actualJson = JSON.stringify(actual);
	const expectedJson = JSON.stringify(expected);
	if (actualJson !== expectedJson) {
		throw new Error(
			`fixture verification failed\nexpected: ${expectedJson}\nactual:   ${actualJson}`,
		);
	}
}
