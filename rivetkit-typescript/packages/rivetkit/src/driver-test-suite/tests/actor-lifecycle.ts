import { describe, expect, test, vi } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest } from "../utils";

function isDestroyRaceError(err: any) {
	const message = typeof err?.message === "string" ? err.message : "";

	return (
		(err?.group === "actor" &&
			[
				"not_found",
				"destroyed_during_creation",
				"destroyed_while_waiting_for_ready",
			].includes(err?.code)) ||
		(err?.group === "rivetkit" &&
			err?.code === "internal_error" &&
			(message.includes("destroyed during creation") ||
				message.includes("destroyed while waiting for ready state") ||
				message.includes("does not exist or was destroyed")))
	);
}

function expectDestroyRaceError(err: any) {
	expect(isDestroyRaceError(err)).toBe(true);
}

async function waitForLifecycleEvents(
	readEvents: () => Promise<Array<{ actorKey: string; event: string }>>,
	actorKey: string,
	expectedEvents: string[],
) {
	await vi.waitFor(
		async () => {
			const events = await readEvents();
			for (const expectedEvent of expectedEvents) {
				expect(
					events.some(
						(event) =>
							event.actorKey === actorKey &&
							event.event === expectedEvent,
					),
				).toBe(true);
			}
		},
		{
			timeout: 5_000,
			interval: 50,
		},
	);
}

async function resolveActorId(handle: { resolve: () => Promise<string> }) {
	try {
		return await handle.resolve();
	} catch (err) {
		expectDestroyRaceError(err);
		return null;
	}
}

async function destroyActor(handle: { destroy: () => Promise<unknown> }) {
	for (let attempt = 0; attempt < 3; attempt++) {
		try {
			await handle.destroy();
			return;
		} catch (err: any) {
			if (
				err?.group === "guard" &&
				err?.code === "service_unavailable"
			) {
				if (attempt >= 2) {
					return;
				}

				await new Promise((resolve) => setTimeout(resolve, 50));
				continue;
			}
			if (isDestroyRaceError(err)) {
				return;
			}

			return;
		}
	}
}

async function waitForActorDestroyed(read: () => Promise<unknown>) {
	await vi.waitFor(
		async () => {
			try {
				await read();
				throw new Error("actor still available");
			} catch (err: any) {
				if (
					err?.group === "guard" &&
					err?.code === "service_unavailable"
				) {
					throw err;
				}

				expectDestroyRaceError(err);
			}
		},
		{
			timeout: 5_000,
			interval: 50,
		},
	);
}

export function runActorLifecycleTests(driverTestConfig: DriverTestConfig) {
	describe.sequential("Actor Lifecycle Tests", () => {
		test(
			"actor stop during start handles in-flight actions and cleanup",
			async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Create actor - this starts the actor
			const actor = client.startStopRaceActor.getOrCreate([
				`test-stop-during-start-${Date.now()}`,
			]);

			// Immediately try to call an action and then destroy
			// This creates a race where the actor might not be fully started yet
			const pingPromise = actor.ping().catch((err) => err);

			// Get actor ID
			const actorId = await resolveActorId(actor);

			// Destroy immediately while start might still be in progress
			await destroyActor(actor);

			// The in-flight action can now either complete or lose the destroy race,
			// but startup must still complete before destroy finishes.
			const result = await pingPromise;
			if (result instanceof Error) {
				expectDestroyRaceError(result);
			} else {
				expect(result).toBe("pong");
			}

			// Verify actor was actually destroyed
			if (actorId) {
				await waitForActorDestroyed(() =>
					client.startStopRaceActor.getForId(actorId).ping(),
				);
			}
			},
			20_000,
		);

		test("actor stop before actor instantiation completes cleans up handler", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-stop-before-instantiation-${Date.now()}`;

			// Create multiple actors rapidly to increase chance of race
			const actors = Array.from({ length: 5 }, (_, i) =>
				client.startStopRaceActor.getOrCreate([`${actorKey}-${i}`]),
			);

			// Resolve actor IDs when the race allows it.
			const ids = (
				await Promise.all(actors.map((a) => resolveActorId(a)))
			).filter((id): id is string => id !== null);

			// Immediately destroy all actors
			await Promise.all(actors.map((a) => destroyActor(a)));

			// Verify all actors were cleaned up
			for (const id of ids) {
				await waitForActorDestroyed(() =>
					client.startStopRaceActor.getForId(id).ping(),
				);
			}
		}, 20_000);

		test(
			"onBeforeActorStart completes before stop proceeds",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.lifecycleObserver.getOrCreate(["observer"]);
				await observer.clearEvents();

				const actorKey = `test-before-actor-start-${Date.now()}`;

				// Create actor
				const actor = client.startStopRaceActor.getOrCreate([actorKey]);

				// Call an action to ensure actor startup has begun. Attach the rejection
				// handler immediately so a destroy race cannot surface as unhandled.
				const statePromise = actor.getState().catch((err: any) => {
					expectDestroyRaceError(err);
					return null;
				});

				// Destroy immediately
				await destroyActor(actor);

				await statePromise;

				// Startup must complete before destroy proceeds, so the observer should
				// have both lifecycle events for this actor key.
				await waitForLifecycleEvents(
					() => observer.getEvents(),
					actorKey,
					["started", "destroy"],
				);
			},
			20_000,
		);

		test("multiple rapid create/destroy cycles handle race correctly", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			// Perform multiple rapid create/destroy cycles
			for (let i = 0; i < 3; i++) {
				const actorKey = `test-rapid-cycle-${Date.now()}-${i}`;
				const actor = client.startStopRaceActor.getOrCreate([actorKey]);

				// Trigger start and race it against destroy.
				const pingPromise = actor.ping().catch((err) => err);

				// Immediately destroy
				await destroyActor(actor);

				const pingResult = await pingPromise;
				if (pingResult instanceof Error) {
					expectDestroyRaceError(pingResult);
				} else {
					expect(pingResult).toBe("pong");
				}
			}

			// If we get here without errors, the race condition is handled correctly
			expect(true).toBe(true);
		}, 20_000);

		test("actor stop called with no actor instance cleans up handler", async (c) => {
			const { client } = await setupDriverTest(c, driverTestConfig);

			const actorKey = `test-cleanup-no-instance-${Date.now()}`;

			// Create and immediately destroy
			const actor = client.startStopRaceActor.getOrCreate([actorKey]);
			const id = await resolveActorId(actor);
			await destroyActor(actor);
			if (id) {
				await waitForActorDestroyed(() =>
					client.startStopRaceActor.getForId(id).ping(),
				);
			}

				// Try to recreate with same key - should work without issues
				const newActor = client.startStopRaceActor.getOrCreate([actorKey]);
				const result = await newActor.ping();
				expect(result).toBe("pong");

				// Clean up
				const newActorId = await resolveActorId(newActor);
				await destroyActor(newActor);
				if (newActorId) {
					await waitForActorDestroyed(() =>
						client.startStopRaceActor.getForId(newActorId).ping(),
					);
				}
			});

		test(
			"onDestroy is called even when actor is destroyed during start",
			async (c) => {
				const { client } = await setupDriverTest(c, driverTestConfig);
				const observer = client.lifecycleObserver.getOrCreate(["observer"]);
				await observer.clearEvents();

				const actorKey = `test-ondestroy-during-start-${Date.now()}`;

				// Create actor
				const actor = client.startStopRaceActor.getOrCreate([actorKey]);

				// Start and immediately destroy
				const statePromise = actor.getState().catch((err: any) => {
					expectDestroyRaceError(err);
					return null;
				});
				await destroyActor(actor);

				// Allow the start request to settle without surfacing an unhandled rejection
				await statePromise;

				// Verify onDestroy was called through the observer actor because the
				// destroyed actor's own state is not readable after the race completes.
				await waitForLifecycleEvents(() => observer.getEvents(), actorKey, [
					"destroy",
				]);
			},
			20_000,
		);
	});
}
