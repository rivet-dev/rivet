import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const ACTOR_NAME = "actor-v2-2-1-migration-bench";
const ENDPOINT = requireEnv("RIVET_ENDPOINT");
const ROWS = Number.parseInt(requireEnv("RIVETKIT_MIGRATION_BENCH_ROWS"), 10);
const VALUE_BYTES = Number.parseInt(
	requireEnv("RIVETKIT_MIGRATION_BENCH_VALUE_BYTES"),
	10,
);
const BATCH_ROWS = 128;

const benchActor = actor({
	state: {
		source: "v2.2.1",
		rows: 0,
		valueBytes: 0,
	},
	actions: {
		seed: async (c) => {
			const value = "x".repeat(VALUE_BYTES);
			for (let start = 0; start < ROWS; start += BATCH_ROWS) {
				const end = Math.min(start + BATCH_ROWS, ROWS);
				await c.kv.putBatch(
					Array.from({ length: end - start }, (_, offset) => {
						const index = start + offset;
						return [`bench-${index.toString().padStart(8, "0")}`, value] as const;
					}),
				);
			}
			c.state.rows = ROWS;
			c.state.valueBytes = VALUE_BYTES;
			await c.saveState({ immediate: true });
			return { actorId: c.actorId, rows: ROWS, valueBytes: VALUE_BYTES };
		},
	},
});

const registry = setup({
	use: { [ACTOR_NAME]: benchActor },
	endpoint: ENDPOINT,
	token: process.env.RIVET_TOKEN ?? "dev",
	namespace: process.env.RIVET_NAMESPACE ?? "default",
	runner: { runnerName: "default" },
	logging: { level: "info" },
});
registry.startRunner();

const client = createClient<typeof registry>({
	endpoint: ENDPOINT,
	token: process.env.RIVET_TOKEN ?? "dev",
	namespace: process.env.RIVET_NAMESPACE ?? "default",
	runnerName: "default",
	encoding: "json",
	devtools: false,
});

const handle = await retry(() => client[ACTOR_NAME].getOrCreate());
const seeded = await retry(() => handle.seed());
console.log(JSON.stringify({ seeded }));
await client.dispose();
await new Promise((resolve) => setTimeout(resolve, 1000));
process.exit(0);

function requireEnv(name: string): string {
	const value = process.env[name];
	if (!value) throw new Error(`${name} is required`);
	return value;
}

async function retry<T>(fn: () => Promise<T>): Promise<T> {
	let lastError: unknown;
	for (let attempt = 0; attempt < 240; attempt++) {
		try {
			return await fn();
		} catch (error) {
			lastError = error;
			await new Promise((resolve) => setTimeout(resolve, 250));
		}
	}
	throw lastError;
}
