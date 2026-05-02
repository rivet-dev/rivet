import assert from "node:assert/strict";

import { actor, setup } from "../../../../../../../rivetkit-typescript/packages/rivetkit/src/mod";
import { createClient } from "../../../../../../../rivetkit-typescript/packages/rivetkit/src/client/mod";
import { db } from "../../../../../../../rivetkit-typescript/packages/rivetkit/src/db/mod";

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
	db: db(),
	actions: {
		scheduled: async (c, payload: unknown) => {
			c.state.scheduledCount++;
			await c.kv.put("scheduled-payload", JSON.stringify(payload));
			await c.db.execute("INSERT INTO items(note) VALUES (?)", "scheduled-ran");
			await c.saveState({ immediate: true });
		},
		verify: async (c) => {
			const binary = await c.kv.get("binary-key", { type: "binary" });
			const manualBatch = await c.kv.listPrefix("manual-batch-");
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
				manualBatch,
				overwrite: await c.kv.get("overwrite-key"),
				deleted: await c.kv.get("delete-key"),
				scheduledPayload: await c.kv.get("scheduled-payload"),
				rows: rows.map((row) => row.note),
				migrations: migrations.map((row) => row.note),
			};
		},
	},
});

main().catch((error) => {
	console.error(error);
	process.exit(1);
});

async function main() {
	const registry = setup({
		use: { [ACTOR_NAME]: baselineActor },
		endpoint: ENDPOINT,
		token: TOKEN,
		namespace: NAMESPACE,
		envoy: { poolName: "default" },
		noWelcome: true,
		logging: { level: "info" },
		shutdown: { disableSignalHandlers: true },
	});
	registry.start();

	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		token: TOKEN,
		namespace: NAMESPACE,
		poolName: "default",
		encoding: "json",
		devtools: false,
		disableMetadataLookup: true,
	});

	const handle = await retry(() => client[ACTOR_NAME].getOrCreate());
	const verified = await retry(async () => {
		const value = await handle.verify();
		assertEqual(value, {
			state: {
				source: "v2.2.1",
				counter: 42,
				migrated: true,
				scheduledCount: 1,
			},
			snapshot: "snapshot-value",
			binary: [1, 3, 3, 7],
			manualBatch: [
				["manual-batch-a", "manual-a"],
				["manual-batch-b", "manual-b"],
			],
			overwrite: "after",
			deleted: null,
			scheduledPayload: JSON.stringify({ source: "v2.2.1", ok: true }),
			rows: ["sqlite-from-v2.2.1", "scheduled-ran"],
			migrations: ["migrated-from-v2.2.1"],
		});
		return value;
	});

	console.log(JSON.stringify({ verified }, null, 2));
	await client.dispose();
	process.exit(0);
}

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
	try {
		assert.deepStrictEqual(actual, expected);
	} catch {
		throw new Error(
			`current RivetKit verification failed\nexpected: ${JSON.stringify(expected)}\nactual:   ${JSON.stringify(actual)}`,
		);
	}
}
