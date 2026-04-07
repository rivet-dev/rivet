import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

export function runDynamicReloadTests(driverTestConfig: DriverTestConfig) {
	describe.skipIf(
		!driverTestConfig.isDynamic || driverTestConfig.skip?.sleep,
	)("Dynamic Actor Reload Tests", () => {
		test("reload forces dynamic actor to sleep and reload on next request", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.sleep.getOrCreate();

			const { startCount: before } = await actor.getCounts();
			expect(before).toBe(1);

			await actor.reload();
			await waitFor(driverTestConfig, 250);

			const { startCount: after } = await actor.getCounts();
			expect(after).toBe(2);
		});
	});
}
