import { promiseWithResolvers } from "@/utils";

/**
 * Possible states for a dynamic actor's host-side runtime lifecycle.
 */
export type DynamicRuntimeState =
	| "inactive"
	| "starting"
	| "running"
	| "failed_start";

/**
 * Host-side runtime status for a dynamic actor.
 *
 * This state is in-memory only and is never persisted. It tracks the actor's
 * startup lifecycle including error metadata, backoff state, and generation
 * counters. Both file-system and engine drivers use this model.
 *
 * Cleared when the actor wrapper is removed during sleep or stop.
 */
export interface DynamicRuntimeStatus {
	state: DynamicRuntimeState;

	// Error metadata from the most recent failed startup attempt.
	lastStartErrorCode: string | undefined;
	lastStartErrorMessage: string | undefined;
	lastStartErrorDetails: string | undefined;
	lastFailureAt: number | undefined;

	// Passive backoff state. No background timers are scheduled; retries
	// happen only when a new request or reload arrives.
	retryAt: number | undefined;
	retryAttempt: number;

	// Reload rate-limit tracking (warning-only).
	reloadCount: number;
	reloadWindowStart: number | undefined;

	// Per-actor monotonic counter incremented synchronously before each startup
	// attempt. Stale async completions compare their captured generation against
	// the current value and discard their result if they differ.
	generation: number;

	// Created via promiseWithResolvers when transitioning to "starting". All
	// concurrent requests join this promise instead of creating a new startup
	// attempt. Resolved on success, rejected on failure.
	startupPromise: {
		promise: Promise<void>;
		resolve: (value: void | PromiseLike<void>) => void;
		reject: (reason?: unknown) => void;
	} | undefined;
}

/**
 * Create a fresh DynamicRuntimeStatus in the "inactive" state.
 */
export function createDynamicRuntimeStatus(): DynamicRuntimeStatus {
	return {
		state: "inactive",
		lastStartErrorCode: undefined,
		lastStartErrorMessage: undefined,
		lastStartErrorDetails: undefined,
		lastFailureAt: undefined,
		retryAt: undefined,
		retryAttempt: 0,
		reloadCount: 0,
		reloadWindowStart: undefined,
		generation: 0,
		startupPromise: undefined,
	};
}

/**
 * Transition to "starting" state. Increments generation synchronously and
 * creates a new startupPromise so concurrent requests can join.
 *
 * The synchronous increment + promise creation ensures that any concurrent
 * request arriving between the transition and the first await always observes
 * the "starting" state and the correct promise.
 */
export function transitionToStarting(
	status: DynamicRuntimeStatus,
): DynamicRuntimeStatus {
	status.state = "starting";
	status.generation += 1;
	status.startupPromise = promiseWithResolvers<void>((reason) => {
		// Swallow unhandled rejection since the caller or a reload may have
		// abandoned this promise intentionally.
		void reason;
	});
	return status;
}

/**
 * Transition to "running" state after a successful startup. Resolves the
 * startupPromise so waiting requests proceed.
 */
export function transitionToRunning(
	status: DynamicRuntimeStatus,
): DynamicRuntimeStatus {
	status.state = "running";
	status.startupPromise?.resolve();
	status.startupPromise = undefined;
	// Reset retry state on success.
	status.retryAt = undefined;
	status.retryAttempt = 0;
	return status;
}

/**
 * Transition to "failed_start" state. Records error metadata and backoff
 * timing, then rejects the startupPromise so waiting requests receive the
 * failure.
 */
export function transitionToFailedStart(
	status: DynamicRuntimeStatus,
	opts: {
		errorCode: string;
		errorMessage: string;
		errorDetails?: string;
		retryAt: number;
	},
): DynamicRuntimeStatus {
	status.state = "failed_start";
	status.lastStartErrorCode = opts.errorCode;
	status.lastStartErrorMessage = opts.errorMessage;
	status.lastStartErrorDetails = opts.errorDetails;
	status.lastFailureAt = Date.now();
	status.retryAt = opts.retryAt;
	status.retryAttempt += 1;
	status.startupPromise?.reject(
		new Error(opts.errorMessage),
	);
	status.startupPromise = undefined;
	return status;
}

/**
 * Reset the status to "inactive". Used when the actor wrapper is removed
 * during sleep/stop or after maxAttempts exhaustion.
 */
export function transitionToInactive(
	status: DynamicRuntimeStatus,
): DynamicRuntimeStatus {
	status.state = "inactive";
	status.lastStartErrorCode = undefined;
	status.lastStartErrorMessage = undefined;
	status.lastStartErrorDetails = undefined;
	status.lastFailureAt = undefined;
	status.retryAt = undefined;
	status.retryAttempt = 0;
	status.reloadCount = 0;
	status.reloadWindowStart = undefined;
	status.startupPromise = undefined;
	// generation is not reset; it only increments.
	return status;
}
