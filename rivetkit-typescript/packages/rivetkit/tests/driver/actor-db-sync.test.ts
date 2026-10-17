import { describe, expect, test } from "vitest";
import { createClient } from "@/client/mod";
import type { registry } from "../../fixtures/driver-test-suite/registry-static";
import { describeDriverMatrix } from "./shared-matrix";
import type { DriverTestConfig } from "./shared-types";

async function startRuntime(config: DriverTestConfig) {
	const runtime = await config.start();
	const client = createClient<typeof registry>({
		endpoint: runtime.endpoint,
		namespace: runtime.namespace,
		poolName: runtime.runnerName,
		encoding: config.encoding,
		disableMetadataLookup: true,
	});
	return {
		client,
		stop: async () => {
			await client.dispose();
			await runtime.cleanup();
		},
	};
}

describeDriverMatrix(
	"Actor Db Sync",
	(driverTestConfig) => {
		describe("synchronous SQLite through the native database provider", () => {
			test("durable sync writes survive sleep and wake", async () => {
				const { client, stop } = await startRuntime(driverTestConfig);
				try {
					const actor = client.dbActorSync.getOrCreate([
						crypto.randomUUID(),
					]);
					await actor.insertSyncAndWaitDurable(["a", "b", "c"]);
					await actor.goToSleep();

					expect(await actor.valuesSync()).toEqual(["a", "b", "c"]);
				} finally {
					await stop();
				}
			});

			test("sync calls are rejected on an actor that did not opt into deferred commits", async () => {
				const { client, stop } = await startRuntime(driverTestConfig);
				try {
					const actor = client.dbActorSyncAwaited.getOrCreate([
						crypto.randomUUID(),
					]);
					await expect(actor.insertSync("a")).rejects.toMatchObject({
						group: "sqlite",
						code: "sync_unsupported",
					});
				} finally {
					await stop();
				}
			});
		});
	},
	{ runtimes: ["native"], sqliteBackends: ["local"], encodings: ["bare"] },
);
