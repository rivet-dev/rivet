#!/usr/bin/env tsx

import { RIVET_ENDPOINT, createActor, destroyActor } from "./utils";

const PARALLEL_WORKERS = 20;
const TEST_DURATION_MS = 15000;

let activeActors: Set<string> = new Set();
let shouldExit = false;
let completedLoops = 0;
let totalLoopDuration = 0;
let startTime = Date.now();

async function actorLoop(workerId: number): Promise<void> {
	while (!shouldExit) {
		const loopStart = Date.now();
		let actorId: string | undefined;

		try {
			const actorResponse = await createActor("default", "test-runner");
			actorId = actorResponse.actor.actor_id;
			activeActors.add(actorId);

			const actorPingResponse = await fetch(`${RIVET_ENDPOINT}/ping`, {
				method: "GET",
				headers: {
					"X-Rivet-Target": "actor",
					"X-Rivet-Actor": actorId,
				},
			});

			if (!actorPingResponse.ok) {
				throw new Error(`Ping failed: ${actorPingResponse.status}`);
			}

			await destroyActor("default", actorId);
			activeActors.delete(actorId);

			const loopDuration = Date.now() - loopStart;
			completedLoops++;
			totalLoopDuration += loopDuration;

		} catch (error) {
			console.error(`Worker ${workerId} error:`, error);
			if (actorId) {
				try {
					await destroyActor("default", actorId);
					activeActors.delete(actorId);
				} catch {}
			}
		}
	}
}

function printProgress() {
	const elapsed = (Date.now() - startTime) / 1000;
	const actorsPerSecond = completedLoops / elapsed;
	const avgLoopDuration = completedLoops > 0 ? totalLoopDuration / completedLoops : 0;

	process.stdout.write(
		`\rElapsed: ${elapsed.toFixed(1)}s | ` +
		`Completed: ${completedLoops} | ` +
		`Actors/sec: ${actorsPerSecond.toFixed(2)} | ` +
		`Avg loop: ${avgLoopDuration.toFixed(0)}ms | ` +
		`Active: ${activeActors.size}`
	);
}

async function cleanup() {
	console.log("\n\nCleaning up active actors...");
	shouldExit = true;

	const cleanupPromises = Array.from(activeActors).map(async (actorId) => {
		try {
			await destroyActor("default", actorId);
			console.log(`Cleaned up actor: ${actorId}`);
		} catch (error) {
			console.error(`Failed to cleanup actor ${actorId}:`, error);
		}
	});

	await Promise.all(cleanupPromises);
	console.log("Cleanup complete");
}

async function main() {
	console.log(`Starting actor stress test...`);
	console.log(`Running ${PARALLEL_WORKERS} workers for ${TEST_DURATION_MS / 1000} seconds\n`);

	process.on("SIGINT", async () => {
		console.log("\nReceived SIGINT");
		await cleanup();
		process.exit(0);
	});

	process.on("SIGTERM", async () => {
		console.log("\nReceived SIGTERM");
		await cleanup();
		process.exit(0);
	});

	const progressInterval = setInterval(printProgress, 100);

	const workers = Array.from({ length: PARALLEL_WORKERS }, (_, i) => actorLoop(i));

	setTimeout(async () => {
		shouldExit = true;
		clearInterval(progressInterval);
		printProgress();
		console.log("\n\nTest duration complete, waiting for workers to finish...");
		await Promise.all(workers);

		const elapsed = (Date.now() - startTime) / 1000;
		console.log("\n=== Final Results ===");
		console.log(`Total runtime: ${elapsed.toFixed(2)}s`);
		console.log(`Total completed loops: ${completedLoops}`);
		console.log(`Average actors/second: ${(completedLoops / elapsed).toFixed(2)}`);
		console.log(`Average loop duration: ${(totalLoopDuration / completedLoops).toFixed(0)}ms`);

		process.exit(0);
	}, TEST_DURATION_MS);

	await Promise.all(workers);
}

main().catch((error) => {
	console.error("Fatal error:", error);
	cleanup().then(() => process.exit(1));
});