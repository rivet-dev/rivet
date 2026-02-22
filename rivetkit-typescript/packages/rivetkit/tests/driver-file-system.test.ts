import { join } from "node:path";
import { createClient } from "@/client/mod";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { describe, expect, test, vi } from "vitest";
import type { registry } from "../fixtures/driver-test-suite/registry";

runDriverTests({
	skip: {
		// Does not support full connection hibernation semantics.
		hibernation: true,
	},
	// TODO: Remove this once timer issues are fixed in actor-sleep.ts
	useRealTimers: true,
	async start() {
		return await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async () => {
				return {
					driver: createFileSystemOrMemoryDriver(
						true,
						{ path: `/tmp/test-${crypto.randomUUID()}` },
					),
				};
			},
		);
	},
});

describe("file-system websocket hibernation cleanup", () => {
	test("calls onDisconnect for restored hibernatable websocket connections", async () => {
		const storagePath = `/tmp/test-${crypto.randomUUID()}`;
		const runtime = await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async () => {
				return {
					driver: createFileSystemOrMemoryDriver(true, {
						path: storagePath,
					}),
				};
			},
		);
		const client = createClient<typeof registry>({
			endpoint: runtime.endpoint,
			namespace: runtime.namespace,
			runnerName: runtime.runnerName,
			disableMetadataLookup: true,
		});
		const conn = client.fileSystemHibernationCleanupActor
			.getOrCreate()
			.connect();

		try {
			expect(await conn.ping()).toBe("pong");
			await conn.triggerSleep();

			// Any action call will wake the actor. This wait ensures the sleep
			// cycle completed before validating disconnect cleanup.
			await vi.waitFor(
				async () => {
					const counts = await client.fileSystemHibernationCleanupActor
						.getOrCreate()
						.getCounts();
					expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
					expect(counts.wakeCount).toBeGreaterThanOrEqual(2);
				},
				{ timeout: 5000, interval: 100 },
			);

			await vi.waitFor(
				async () => {
					const disconnectWakeCounts =
						await client.fileSystemHibernationCleanupActor
							.getOrCreate()
							.getDisconnectWakeCounts();
					expect(disconnectWakeCounts).toEqual([2]);
				},
				{ timeout: 5000, interval: 100 },
			);
		} finally {
			await conn.dispose().catch(() => undefined);
			await client.dispose().catch(() => undefined);
			await runtime.cleanup();
		}
	});
});
