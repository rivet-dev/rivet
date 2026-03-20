import { getLogger } from "@/common/log";
import { ActorError } from "@/actor/errors";
import type { DynamicRuntimeStatus } from "./runtime-status";
import {
	transitionToStarting,
	transitionToRunning,
	transitionToFailedStart,
} from "./runtime-status";
import type { DynamicStartupOptions } from "./internal";
import { DYNAMIC_STARTUP_DEFAULTS } from "./internal";

function logger() {
	return getLogger("dynamic-startup");
}

/**
 * Compute the next retry delay using exponential backoff.
 *
 * Formula: min(maxDelay, initialDelay * multiplier^attempt)
 * with optional uniform jitter in [delay*0.5, delay).
 */
export function computeBackoffDelay(
	attempt: number,
	options: Required<DynamicStartupOptions>,
): number {
	const delay = Math.min(
		options.retryMaxDelayMs,
		options.retryInitialDelayMs * options.retryMultiplier ** attempt,
	);
	if (options.retryJitter) {
		return delay * (0.5 + Math.random() * 0.5);
	}
	return delay;
}

/**
 * Orchestrate a dynamic actor startup attempt with coalescing and generation
 * tracking.
 *
 * When the status is "starting", concurrent callers await the existing
 * startupPromise instead of launching a new attempt. When the status is
 * "inactive" or "failed_start" with expired backoff, a new attempt begins.
 *
 * The startupPromise is created synchronously (inside transitionToStarting)
 * before any async work begins. This ensures that any concurrent request
 * arriving between the synchronous transition and the first await always
 * observes the "starting" state and joins the correct promise. Without this
 * synchronous guarantee, two callers could both read "inactive", both decide
 * to start, and launch duplicate attempts.
 *
 * Generation invalidation prevents stale async completions from corrupting
 * the status. Each startup attempt captures the generation at the time of
 * the synchronous transition. If a reload or another retry increments the
 * generation while the original attempt is still in flight, the original
 * attempt's completion handler detects the mismatch and discards its result
 * instead of overwriting the newer attempt's state.
 */
export async function coalesceDynamicStartup(
	status: DynamicRuntimeStatus,
	startupFn: () => Promise<void>,
	options?: Required<DynamicStartupOptions>,
): Promise<void> {
	const opts = options ?? DYNAMIC_STARTUP_DEFAULTS;

	// Already running. Nothing to do.
	if (status.state === "running") {
		return;
	}

	// Concurrent requests during an in-flight startup attempt join the
	// existing promise instead of launching a duplicate attempt.
	if (status.state === "starting" && status.startupPromise) {
		await status.startupPromise.promise;
		return;
	}

	// During active backoff, return the stored failure immediately without
	// attempting startup.
	if (status.state === "failed_start" && status.retryAt !== undefined) {
		if (Date.now() < status.retryAt) {
			throw new ActorError(
				"dynamic",
				status.lastStartErrorCode ?? "dynamic_startup_failed",
				status.lastStartErrorMessage ??
					"Dynamic actor startup failed",
				{ public: true },
			);
		}
	}

	// Synchronously transition to "starting" before any async work.
	// This increments the generation and creates startupPromise in the
	// same synchronous tick. Any concurrent request arriving after this
	// point will see state === "starting" and join the promise above.
	transitionToStarting(status);
	const capturedGeneration = status.generation;

	try {
		await startupFn();

		// A newer generation (from a reload or retry) has superseded this
		// attempt. Discard this completion to avoid overwriting the newer
		// attempt's state.
		if (status.generation !== capturedGeneration) {
			logger().debug({
				msg: "discarding stale startup completion",
				capturedGeneration,
				currentGeneration: status.generation,
			});
			return;
		}

		transitionToRunning(status);
	} catch (error) {
		// A newer generation has superseded this attempt. Discard this
		// failure so it does not corrupt the newer attempt's state.
		if (status.generation !== capturedGeneration) {
			logger().debug({
				msg: "discarding stale startup failure",
				capturedGeneration,
				currentGeneration: status.generation,
			});
			return;
		}

		const errorCode =
			error instanceof ActorError
				? error.code
				: "dynamic_startup_failed";
		const errorMessage =
			error instanceof Error ? error.message : String(error);
		const errorDetails =
			error instanceof Error ? error.stack : undefined;
		const backoffDelay = computeBackoffDelay(status.retryAttempt, opts);

		transitionToFailedStart(status, {
			errorCode,
			errorMessage,
			errorDetails,
			retryAt: Date.now() + backoffDelay,
		});

		throw error;
	}
}
