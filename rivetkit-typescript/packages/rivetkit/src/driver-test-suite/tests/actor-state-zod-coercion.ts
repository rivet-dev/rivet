import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";

const SLEEP_WAIT_MS = 150;

export function runActorStateZodCoercionTests(
	driverTestConfig: DriverTestConfig,
) {
	describe("Actor State Zod Coercion Tests", () => {
		test("preserves state through sleep/wake with Zod coercion", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.stateZodCoercionActor.getOrCreate([
				`zod-roundtrip-${crypto.randomUUID()}`,
			]);

			await actor.setCount(42);
			await actor.setLabel("custom");

			// Sleep and wake
			await actor.triggerSleep();
			await waitFor(driverTestConfig, SLEEP_WAIT_MS);

			const state = await actor.getState();
			expect(state.count).toBe(42);
			expect(state.label).toBe("custom");
		});

		test("Zod defaults fill missing fields on wake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.stateZodCoercionActor.getOrCreate([
				`zod-defaults-${crypto.randomUUID()}`,
			]);

			// Initial state should have defaults from the schema
			const state = await actor.getState();
			expect(state.count).toBe(0);
			expect(state.label).toBe("default");
		});

		test("Zod coercion preserves values after mutation and wake", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);
			const actor = client.stateZodCoercionActor.getOrCreate([
				`zod-mutate-wake-${crypto.randomUUID()}`,
			]);

			await actor.setCount(99);
			await actor.setLabel("updated");

			// Sleep
			await actor.triggerSleep();
			await waitFor(driverTestConfig, SLEEP_WAIT_MS);

			// Wake and verify Zod parse preserved values
			const state = await actor.getState();
			expect(state.count).toBe(99);
			expect(state.label).toBe("updated");

			// Mutate again and verify
			await actor.setLabel("second-update");
			const state2 = await actor.getState();
			expect(state2.label).toBe("second-update");
		});
	});
}
