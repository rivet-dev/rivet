import { randomUUID } from "node:crypto";
import { createClient } from "rivetkit/client";
import type { Registry } from "../server/registry";
import { type SmokeTestError, spawnActor } from "./spawn-actor";

function parseEnvInt(value: string | undefined, fallback: number) {
	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed <= 0) {
		return fallback;
	}
	return Math.floor(parsed);
}

/**
 * What the actor should do once spawned.
 */
export type ActorBehavior = "sleep-cycle" | "http";

const RUN_DURATION = parseEnvInt(process.env.RUN_DURATION, 10_000);
const SPAWN_ACTOR_INTERVAL = parseEnvInt(process.env.SPAWN_ACTOR_INTERVAL, 10);
const TOTAL_ACTOR_COUNT = Math.ceil(RUN_DURATION / SPAWN_ACTOR_INTERVAL);
export const BEHAVIOR: ActorBehavior = (process.env.BEHAVIOR ??
	"sleep-cycle") as ActorBehavior;
const PROGRESS_LOG_INTERVAL_MS = 250;

type DurationStats = {
	average: number;
	median: number;
	min: number;
	max: number;
};

async function delay(ms: number) {
	return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

function calculateDurationStats(durations: number[]): DurationStats {
	if (durations.length === 0) {
		return { average: 0, median: 0, min: 0, max: 0 };
	}

	const sorted = [...durations].sort((left, right) => left - right);
	const count = sorted.length;
	const sum = sorted.reduce(
		(runningSum, duration) => runningSum + duration,
		0,
	);
	const average = sum / count;
	const min = sorted[0];
	const max = sorted[count - 1];
	const median =
		count % 2 === 0
			? (sorted[count / 2 - 1] + sorted[count / 2]) / 2
			: sorted[Math.floor(count / 2)];

	return { average, median, min, max };
}

function logProgress({
	totalActorCount,
	startedCount,
	successCount,
	failureCount,
	iterationDurations,
}: {
	totalActorCount: number;
	startedCount: number;
	successCount: number;
	failureCount: number;
	iterationDurations: number[];
}) {
	const stats = calculateDurationStats(iterationDurations);
	const remainingCount = totalActorCount - startedCount;
	const pendingCount = Math.max(
		0,
		startedCount - (successCount + failureCount),
	);
	console.log(
		`progress: success=${successCount}, failures=${failureCount}, pending=${pendingCount}, remaining=${remainingCount}, samples=${iterationDurations.length}, duration(ms): avg=${stats.average.toFixed(2)}, median=${stats.median.toFixed(2)}, min=${stats.min.toFixed(2)}, max=${stats.max.toFixed(2)}`,
	);
}

async function main() {
	const client = createClient<Registry>();
	const testId = randomUUID();
	const errors: SmokeTestError[] = [];
	let successCount = 0;
	let failureCount = 0;
	let startedCount = 0;
	const iterationDurations: number[] = [];
	const logProgressTick = () =>
		logProgress({
			totalActorCount: TOTAL_ACTOR_COUNT,
			startedCount,
			successCount,
			failureCount,
			iterationDurations,
		});

	console.log(
		`starting smoke test (run duration: ${RUN_DURATION}ms, interval: ${SPAWN_ACTOR_INTERVAL}ms, expected actors: ${TOTAL_ACTOR_COUNT}, test id: ${testId})`,
	);

	const pendingActors: Promise<void>[] = [];
	const progressInterval = setInterval(
		logProgressTick,
		PROGRESS_LOG_INTERVAL_MS,
	);
	logProgressTick();

	try {
		for (let index = 0; index < TOTAL_ACTOR_COUNT; index++) {
			startedCount += 1;
			pendingActors.push(
				spawnActor({
					client,
					index,
					testId,
					errors,
					iterationDurations,
					onSuccess: () => {
						successCount += 1;
					},
					onFailure: () => {
						failureCount += 1;
					},
				}),
			);
			if (index < TOTAL_ACTOR_COUNT - 1) {
				await delay(SPAWN_ACTOR_INTERVAL);
			}
		}

		await Promise.all(pendingActors);
	} finally {
		clearInterval(progressInterval);
	}

	logProgressTick();

	const finalStats = calculateDurationStats(iterationDurations);
	console.log(
		`iteration duration stats (ms): samples=${iterationDurations.length}, avg=${finalStats.average.toFixed(2)}, median=${finalStats.median.toFixed(2)}, min=${finalStats.min.toFixed(2)}, max=${finalStats.max.toFixed(2)}`,
	);

	if (errors.length > 0) {
		console.error(`completed with ${errors.length} error(s)`);
		errors.forEach(({ index, error }) => {
			console.error(`[${index}] captured error`, error);
		});
		process.exitCode = 1;
		return;
	}

	console.log("smoke test completed successfully");
}

main().catch((error) => {
	console.error("fatal error", error);
	process.exit(1);
});
