import assert from "node:assert/strict";

import { actor, setup } from "../../../../../../../rivetkit-typescript/packages/rivetkit/src/mod";
import { createClient } from "../../../../../../../rivetkit-typescript/packages/rivetkit/src/client/mod";

const ACTOR_NAME = "actor-v2-2-1-migration-bench";
const ENDPOINT = requireEnv("RIVET_ENDPOINT");
const ROWS = Number.parseInt(requireEnv("RIVETKIT_MIGRATION_BENCH_ROWS"), 10);
const VALUE_BYTES = Number.parseInt(
	requireEnv("RIVETKIT_MIGRATION_BENCH_VALUE_BYTES"),
	10,
);

const benchActor = actor({
	state: {
		source: "v2.2.1",
		rows: 0,
		valueBytes: 0,
	},
	actions: {
		verify: async (c) => ({
			state: c.state,
			first: await c.kv.get("bench-00000000"),
			last: await c.kv.get(`bench-${(ROWS - 1).toString().padStart(8, "0")}`),
		}),
	},
});

main().catch((error) => {
	console.error(error);
	process.exit(1);
});

async function main() {
	const registry = setup({
		use: { [ACTOR_NAME]: benchActor },
		endpoint: ENDPOINT,
		token: process.env.RIVET_TOKEN ?? "dev",
		namespace: process.env.RIVET_NAMESPACE ?? "default",
		envoy: { poolName: "default" },
		noWelcome: true,
		logging: { level: "debug" },
		shutdown: { disableSignalHandlers: true },
	});
	registry.start();
	// Envoy registration is asynchronous and `start()` intentionally returns
	// immediately. Keep that process-registration race outside the actor startup
	// measurement; otherwise a request can spend 30 seconds in Guard's ready
	// timeout before migration even begins.
	await new Promise((resolve) => setTimeout(resolve, 2_000));

	const client = createClient<typeof registry>({
		endpoint: ENDPOINT,
		token: process.env.RIVET_TOKEN ?? "dev",
		namespace: process.env.RIVET_NAMESPACE ?? "default",
		poolName: "default",
		encoding: "json",
		devtools: false,
		disableMetadataLookup: true,
	});

	const startedAt = performance.now();
	const handle = await retry(() => client[ACTOR_NAME].getOrCreate());
	const verified = await retry(() => handle.verify());
	const elapsedMs = performance.now() - startedAt;
	assert.deepEqual(verified.state, {
		source: "v2.2.1",
		rows: ROWS,
		valueBytes: VALUE_BYTES,
	});
	assert.equal(verified.first?.length, VALUE_BYTES);
	assert.equal(verified.last?.length, VALUE_BYTES);
	const actionStartedAt = performance.now();
	await handle.verify();
	const actionRoundTripMs = performance.now() - actionStartedAt;
	console.log(
		JSON.stringify({
			migrationBench: {
				rows: ROWS,
				valueBytes: VALUE_BYTES,
				inputBytes: ROWS * VALUE_BYTES,
				startupAndVerifyMs: elapsedMs,
				actionRoundTripMs,
			},
		}),
	);
	await client.dispose();
	await registry.shutdown();
	process.exit(0);
}

function requireEnv(name: string): string {
	const value = process.env[name];
	if (!value) throw new Error(`${name} is required`);
	return value;
}

async function retry<T>(fn: () => Promise<T>): Promise<T> {
	let lastError: unknown;
	for (let attempt = 0; attempt < 360; attempt++) {
		try {
			return await fn();
		} catch (error) {
			lastError = error;
			await new Promise((resolve) => setTimeout(resolve, 250));
		}
	}
	throw lastError;
}
