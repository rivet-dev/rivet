import { describeDriverMatrix } from "./shared-matrix";
import { describe, expect, test } from "vitest";
import { setupDriverTest, waitFor } from "./shared-utils";

const POLL_INTERVAL_MS = 25;
const POLL_ATTEMPTS = 40;
const REAL_TIMER_POLL_INTERVAL_MS = 50;
const REAL_TIMER_POLL_ATTEMPTS = 600;
const DEFERRED_SAVE_WAIT_MS = 150;

async function waitForObserverPhase(input: {
	actorKey: string;
	driverTestConfig: Parameters<typeof describeDriverMatrix>[1] extends (
		driverTestConfig: infer T,
	) => void
		? T
		: never;
	observer: Awaited<ReturnType<typeof setupDriverTest>>["client"]["saveStateObserver"];
	expectedPhase: string;
	pollAttempts: number;
	pollIntervalMs: number;
}) {
	let phase: string | null = null;

	for (let i = 0; i < input.pollAttempts; i++) {
		phase = await input.observer.getPhase(input.actorKey);
		if (phase === input.expectedPhase) {
			return;
		}

		await waitFor(input.driverTestConfig, input.pollIntervalMs);
	}

	expect(phase).toBe(input.expectedPhase);
}

async function waitForPersistedValue(input: {
	actor: Awaited<ReturnType<typeof setupDriverTest>>["client"]["saveStateActor"];
	driverTestConfig: Parameters<typeof describeDriverMatrix>[1] extends (
		driverTestConfig: infer T,
	) => void
		? T
		: never;
	expectedValue: number;
	pollAttempts: number;
	pollIntervalMs: number;
}) {
	let lastValue: number | undefined;

	for (let i = 0; i < input.pollAttempts; i++) {
		try {
			lastValue = await input.actor.getValue();
		} catch {
			lastValue = undefined;
		}

		if (lastValue === input.expectedValue) {
			return;
		}

		await waitFor(input.driverTestConfig, input.pollIntervalMs);
	}

	expect(lastValue).toBe(input.expectedValue);
}

describeDriverMatrix("Actor Save State", (driverTestConfig) => {
	const pollAttempts = driverTestConfig.useRealTimers
		? REAL_TIMER_POLL_ATTEMPTS
		: POLL_ATTEMPTS;
	const pollIntervalMs = driverTestConfig.useRealTimers
		? REAL_TIMER_POLL_INTERVAL_MS
		: POLL_INTERVAL_MS;

	describe("Actor Save State Tests", () => {
		test("saveState({ immediate: true }) persists before a hard crash", async (c) => {
			const { client, hardCrashActor, hardCrashPreservesData } =
				await setupDriverTest(c, driverTestConfig);
			if (!hardCrashPreservesData) {
				return;
			}
			if (!hardCrashActor) {
				throw new Error(
					"hardCrashActor test helper is unavailable for this driver",
				);
			}

			const actorKey = `save-immediate-${crypto.randomUUID()}`;
			const actor = client.saveStateActor.getOrCreate([actorKey]);
			const observer = client.saveStateObserver.getOrCreate(["observer"]);
			await observer.reset(actorKey);

			const actorId = await actor.resolve();
			const pending = actor.saveImmediateAndBlock(41);
			void pending.catch(() => undefined);

			await waitForObserverPhase({
				actorKey,
				driverTestConfig,
				observer,
				expectedPhase: "immediate",
				pollAttempts,
				pollIntervalMs,
			});

			await hardCrashActor(actorId);

			await waitForPersistedValue({
				actor,
				driverTestConfig,
				expectedValue: 41,
				pollAttempts,
				pollIntervalMs,
			});
		});

		test("saveState({ maxWait }) persists once the deadline elapses", async (c) => {
			const { client, hardCrashActor, hardCrashPreservesData } =
				await setupDriverTest(c, driverTestConfig);
			if (!hardCrashPreservesData) {
				return;
			}
			if (!hardCrashActor) {
				throw new Error(
					"hardCrashActor test helper is unavailable for this driver",
				);
			}

			const actorKey = `save-deferred-${crypto.randomUUID()}`;
			const actor = client.saveStateActor.getOrCreate([actorKey]);
			const observer = client.saveStateObserver.getOrCreate(["observer"]);
			await observer.reset(actorKey);

			const actorId = await actor.resolve();
			const pending = actor.saveDeferredAndBlock(73, 100);
			void pending.catch(() => undefined);

			await waitForObserverPhase({
				actorKey,
				driverTestConfig,
				observer,
				expectedPhase: "deferred",
				pollAttempts,
				pollIntervalMs,
			});

			await waitFor(driverTestConfig, DEFERRED_SAVE_WAIT_MS);
			await hardCrashActor(actorId);

			await waitForPersistedValue({
				actor,
				driverTestConfig,
				expectedValue: 73,
				pollAttempts,
				pollIntervalMs,
			});
		});
	});
});
