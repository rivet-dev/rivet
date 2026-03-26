import { join } from "node:path";
import { createClient } from "@/client/mod";
import { createTestRuntime } from "@/driver-test-suite/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { describe, expect, test, vi } from "vitest";
import type { registry } from "../fixtures/db-closed-race/registry";
import { collectedErrors } from "../fixtures/db-closed-race/registry";

describe("database closed race condition", () => {
	test("setInterval tick gets actionable error after destroy", async () => {
		const runtime = await createTestRuntime(
			join(__dirname, "../fixtures/db-closed-race/registry.ts"),
			async () => {
				return {
					driver: createFileSystemOrMemoryDriver(true, {
						path: `/tmp/test-db-closed-race-${crypto.randomUUID()}`,
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

		// Clear any errors from previous runs
		collectedErrors.length = 0;

		try {
			const actor = client.dbClosedRaceActor.getOrCreate([
				`race-${crypto.randomUUID()}`,
			]);

			// Wait for ticks to confirm the interval is running and db works
			await vi.waitFor(
				async () => {
					const count = await actor.getTickCount();
					expect(count).toBeGreaterThanOrEqual(3);
				},
				{ timeout: 5000, interval: 50 },
			);

			// No errors before destroy
			expect(collectedErrors).toHaveLength(0);

			// Destroy the actor. The cached db reference is now closed,
			// but the interval keeps firing.
			await actor.destroy();

			// Wait for orphaned interval ticks to hit the closed database
			await new Promise((resolve) => setTimeout(resolve, 300));

			// The orphaned interval should have hit the improved error
			expect(collectedErrors.length).toBeGreaterThan(0);
			expect(
				collectedErrors.some((e) => e.includes("Database is closed")),
			).toBe(true);
			expect(
				collectedErrors.some((e) => e.includes("c.abortSignal")),
			).toBe(true);
		} finally {
			await client.dispose().catch(() => undefined);
			await runtime.cleanup();
		}
	});
});
