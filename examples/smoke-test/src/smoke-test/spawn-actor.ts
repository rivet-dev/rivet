import { performance } from "node:perf_hooks";
import type { createClient } from "rivetkit/client";
import type { Registry } from "../server/registry";

export type SmokeTestError = {
	index: number;
	error: unknown;
};

export type RegistryClient = ReturnType<typeof createClient<Registry>>;

export type SpawnActorOptions = {
	client: RegistryClient;
	index: number;
	testId: string;
	errors: SmokeTestError[];
	iterationDurations: number[];
	onSuccess: () => void;
	onFailure: () => void;
};

export async function spawnActor({
	client,
	index,
	testId,
	errors,
	iterationDurations,
	onSuccess,
	onFailure,
}: SpawnActorOptions): Promise<void> {
	let succeeded = false;

	try {
		for (let i = 0; i < 20; i++) {
			// Connect to actor
			const connMethod = Math.random() > 0.5 ? "http" : "websocket";
			const iterationStart = performance.now();
			if (connMethod === "websocket") {
				const key = ["test", testId, index.toString()];
				const counter = client.counter.getOrCreate(key).connect();
				await counter.increment(1);
				await counter.dispose();
			} else if (connMethod === "http") {
				const key = ["test", testId, index.toString()];
				const counter = client.counter.getOrCreate(key);
				await counter.increment(1);
			}
			const iterationEnd = performance.now();
			const iterationDuration = iterationEnd - iterationStart;
			iterationDurations.push(iterationDuration);

			// Wait for actor to sleep (if > 500 ms)
			const sleepTime = 100 + Math.random() * 800;
			console.log("sleeping", sleepTime);
			// const sleepTime = 1000;
			await new Promise((res) => setTimeout(res, sleepTime));
		}

		succeeded = true;
		onSuccess();
	} catch (error) {
		errors.push({ index, error });
		onFailure();
	}

	if (succeeded) {
	}
}
