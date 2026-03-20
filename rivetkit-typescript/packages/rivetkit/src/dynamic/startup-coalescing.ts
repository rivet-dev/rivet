import { getLogger } from "@/common/log";
import { ActorError, DynamicLoadTimeout } from "@/actor/errors";
import { isDev } from "@/utils/env-vars";
import type { DynamicRuntimeStatus } from "./runtime-status";
import {
	transitionToStarting,
	transitionToRunning,
	transitionToFailedStart,
	transitionToInactive,
} from "./runtime-status";
import type { DynamicStartupOptions } from "./internal";
import { DYNAMIC_STARTUP_DEFAULTS } from "./internal";

function logger() {
	return getLogger("dynamic-startup");
}

export const SANITIZED_STARTUP_MESSAGE =
	"Dynamic actor startup failed. Check server logs for details.";

/**
 * Create an ActorError for dynamic startup failures with environment-aware
 * sanitization.
 *
 * Production errors are sanitized to prevent leaking internal details
 * (stack traces, loader output, file paths) to clients. The error code is
 * always included so clients can programmatically distinguish failure types.
 * Development errors include the full message and details so developers can
 * debug without checking server logs.
 *
 * Full details are always emitted to server logs regardless of environment.
 */
function createSanitizedStartupError(
	errorCode: string,
	errorMessage: string,
	errorDetails?: string,
): ActorError {
	// Always log full details to server logs in all environments.
	logger().error({
		msg: "dynamic actor startup failed",
		errorCode,
		errorMessage,
		errorDetails,
	});

	const error = new ActorError(
		"dynamic",
		errorCode,
		isDev() ? errorMessage : SANITIZED_STARTUP_MESSAGE,
		{
			public: true,
			metadata: isDev() && errorDetails ? { details: errorDetails } : undefined,
		},
	);
	error.statusCode = 503;
	return error;
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
 *
 * An AbortController is created for each startup attempt and its signal
 * passed to the startupFn. A timeout (configured via options.timeoutMs)
 * aborts the controller if startup does not complete in time. The resulting
 * DynamicLoadTimeout error participates in the normal backoff flow.
 */
export async function coalesceDynamicStartup(
	status: DynamicRuntimeStatus,
	startupFn: (signal: AbortSignal) => Promise<void>,
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

	// Backoff is passive: no background timers or retry loops are scheduled.
	// Retries only happen when an incoming request or explicit reload arrives.
	// This prevents failed actors from spinning in memory indefinitely and
	// keeps resource usage proportional to actual demand.
	if (status.state === "failed_start" && status.retryAt !== undefined) {
		if (Date.now() < status.retryAt) {
			throw createSanitizedStartupError(
				status.lastStartErrorCode ?? "dynamic_startup_failed",
				status.lastStartErrorMessage ?? "Dynamic actor startup failed",
				status.lastStartErrorDetails,
			);
		}
	}

	// Synchronously transition to "starting" before any async work.
	// This increments the generation and creates startupPromise in the
	// same synchronous tick. Any concurrent request arriving after this
	// point will see state === "starting" and join the promise above.
	transitionToStarting(status);
	const capturedGeneration = status.generation;

	// Create an AbortController for this startup attempt so that the
	// loader and internal async operations can be cancelled on timeout
	// or reload.
	const abortController = new AbortController();
	status.abortController = abortController;
	const timeoutMs = opts.timeoutMs;
	const timeoutHandle = setTimeout(() => {
		abortController.abort(new DynamicLoadTimeout(timeoutMs));
	}, timeoutMs);

	try {
		await startupFn(abortController.signal);

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
		// If the AbortController was aborted due to timeout, wrap the
		// error as a DynamicLoadTimeout so the backoff flow treats it
		// consistently.
		const effectiveError =
			abortController.signal.aborted &&
			abortController.signal.reason instanceof DynamicLoadTimeout
				? abortController.signal.reason
				: error;

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
			effectiveError instanceof ActorError
				? effectiveError.code
				: "dynamic_startup_failed";
		const errorMessage =
			effectiveError instanceof Error
				? effectiveError.message
				: String(effectiveError);
		const errorDetails =
			effectiveError instanceof Error
				? effectiveError.stack
				: undefined;
		const backoffDelay = computeBackoffDelay(status.retryAttempt, opts);

		transitionToFailedStart(status, {
			errorCode,
			errorMessage,
			errorDetails,
			retryAt: Date.now() + backoffDelay,
		});

		// When maxAttempts is exceeded, transition to inactive so the host
		// wrapper is torn down and the next request starts a fresh attempt
		// from attempt 0. maxAttempts of 0 means unlimited retries.
		if (opts.maxAttempts > 0 && status.retryAttempt >= opts.maxAttempts) {
			logger().warn({
				msg: "max startup attempts exhausted, transitioning to inactive",
				retryAttempt: status.retryAttempt,
				maxAttempts: opts.maxAttempts,
			});
			transitionToInactive(status);
		}

		throw createSanitizedStartupError(errorCode, errorMessage, errorDetails);
	} finally {
		clearTimeout(timeoutHandle);
		status.abortController = undefined;
	}
}
