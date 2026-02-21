import { describe, expect, test } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

export function runActorLifecycleTests(driverTestConfig: DriverTestConfig) {
	describe("Actor Lifecycle Tests", () => {
		test("actor stop during start waits for start to complete", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-stop-during-start-${Date.now()}`;

			// Create actor - this starts the actor
			const actor = client.startStopRaceActor.getOrCreate([actorKey]);

			// Immediately try to call an action and then destroy
			// This creates a race where the actor might not be fully started yet
			const pingPromise = actor.ping();

			// Get actor ID
			const actorId = await actor.resolve();

			// Destroy immediately while start might still be in progress
			await actor.destroy();

			// The ping should still complete successfully because destroy waits for start
			const result = await pingPromise;
			expect(result).toBe("pong");

			// Verify actor was actually destroyed
			let destroyed = false;
			try {
				await client.startStopRaceActor.getForId(actorId).ping();
			} catch (err: any) {
				destroyed = true;
				expect(err.group).toBe("actor");
				expect(err.code).toBe("not_found");
			}
			expect(destroyed).toBe(true);
		});

		test("actor stop before actor instantiation completes cleans up handler", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-stop-before-instantiation-${Date.now()}`;

			// Create multiple actors rapidly to increase chance of race
			const actors = Array.from({ length: 5 }, (_, i) =>
				client.startStopRaceActor.getOrCreate([
					`${actorKey}-${i}`,
				]),
			);

			// Resolve all actor IDs (this triggers start)
			const ids = await Promise.all(actors.map((a) => a.resolve()));

			// Immediately destroy all actors
			await Promise.all(actors.map((a) => a.destroy()));

			// Verify all actors were cleaned up
			for (const id of ids) {
				let destroyed = false;
				try {
					await client.startStopRaceActor.getForId(id).ping();
				} catch (err: any) {
					destroyed = true;
					expect(err.group).toBe("actor");
					expect(err.code).toBe("not_found");
				}
				expect(destroyed, `actor ${id} should be destroyed`).toBe(
					true,
				);
			}
		});

		test("onBeforeActorStart completes before stop proceeds", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-before-actor-start-${Date.now()}`;

			// Create actor
			const actor = client.startStopRaceActor.getOrCreate([actorKey]);

			// Call action to ensure actor is starting
			const statePromise = actor.getState();

			// Destroy immediately
			await actor.destroy();

			// State should be initialized because onBeforeActorStart must complete
			const state = await statePromise;
			expect(state.initialized).toBe(true);
			expect(state.startCompleted).toBe(true);
		});

		test("multiple rapid create/destroy cycles handle race correctly", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Perform multiple rapid create/destroy cycles
			for (let i = 0; i < 10; i++) {
				const actorKey = `test-rapid-cycle-${Date.now()}-${i}`;
				const actor = client.startStopRaceActor.getOrCreate([
					actorKey,
				]);

				// Trigger start
				const resolvePromise = actor.resolve();

				// Immediately destroy
				const destroyPromise = actor.destroy();

				// Both should complete without errors
				await Promise.all([resolvePromise, destroyPromise]);
			}

			// If we get here without errors, the race condition is handled correctly
			expect(true).toBe(true);
		});

		test("actor stop called with no actor instance cleans up handler", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-cleanup-no-instance-${Date.now()}`;

			// Create and immediately destroy
			const actor = client.startStopRaceActor.getOrCreate([actorKey]);
			const id = await actor.resolve();
			await actor.destroy();

			// Try to recreate with same key - should work without issues
			const newActor = client.startStopRaceActor.getOrCreate([
				actorKey,
			]);
			const result = await newActor.ping();
			expect(result).toBe("pong");

			// Clean up
			await newActor.destroy();
		});

		test("onDestroy is called even when actor is destroyed during start", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-ondestroy-during-start-${Date.now()}`;

			// Create actor
			const actor = client.startStopRaceActor.getOrCreate([actorKey]);

			// Start and immediately destroy
			const statePromise = actor.getState();
			await actor.destroy();

			// Verify onDestroy was called (requires actor to be started)
			const state = await statePromise;
			expect(state.destroyCalled).toBe(true);
		});
	});
}
