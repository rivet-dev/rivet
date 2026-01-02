/**
 * Thrown from steps to prevent retry.
 * Use this when an error is unrecoverable and retrying would be pointless.
 */
export class CriticalError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "CriticalError";
	}
}

/**
 * Thrown from steps to force rollback without retry.
 */
export class RollbackError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "RollbackError";
	}
}

/**
 * Thrown when rollback is used without a checkpoint.
 */
export class RollbackCheckpointError extends Error {
	constructor() {
		super("Rollback requires a checkpoint before any rollback step");
		this.name = "RollbackCheckpointError";
	}
}

/**
 * Internal: Workflow should sleep until deadline.
 * This is thrown to yield control back to the scheduler.
 */
export class SleepError extends Error {
	constructor(public readonly deadline: number) {
		super(`Sleeping until ${deadline}`);
		this.name = "SleepError";
	}
}

/**
 * Internal: Workflow is waiting for messages.
 * This is thrown to yield control back to the scheduler.
 */
export class MessageWaitError extends Error {
	constructor(public readonly messageNames: string[]) {
		super(`Waiting for messages: ${messageNames.join(", ")}`);
		this.name = "MessageWaitError";
	}
}

/**
 * Internal: Workflow was evicted.
 * This is thrown when the workflow is being gracefully stopped.
 */
export class EvictedError extends Error {
	constructor() {
		super("Workflow evicted");
		this.name = "EvictedError";
	}
}

/**
 * Internal: Stop rollback traversal.
 */
export class RollbackStopError extends Error {
	constructor() {
		super("Rollback traversal halted");
		this.name = "RollbackStopError";
	}
}

/**
 * Workflow code changed incompatibly.
 * Thrown when history doesn't match the current workflow code.
 */
export class HistoryDivergedError extends Error {
	constructor(message: string) {
		super(message);
		this.name = "HistoryDivergedError";
	}
}

/**
 * Step exhausted all retries.
 */
export class StepExhaustedError extends Error {
	constructor(
		public readonly stepName: string,
		public readonly lastError?: string,
	) {
		super(
			`Step "${stepName}" exhausted retries: ${lastError ?? "unknown error"}`,
		);
		this.name = "StepExhaustedError";
	}
}

/**
 * Step failed (will be retried).
 * Internal error used to trigger retry logic.
 */
export class StepFailedError extends Error {
	constructor(
		public readonly stepName: string,
		public readonly originalError: unknown,
		public readonly attempts: number,
	) {
		super(`Step "${stepName}" failed (attempt ${attempts})`);
		this.name = "StepFailedError";
		this.cause = originalError;
	}
}

/**
 * Join had branch failures.
 */
export class JoinError extends Error {
	constructor(public readonly errors: Record<string, Error>) {
		super(`Join failed: ${Object.keys(errors).join(", ")}`);
		this.name = "JoinError";
	}
}

/**
 * Race had all branches fail.
 */
export class RaceError extends Error {
	constructor(
		message: string,
		public readonly errors: Array<{ name: string; error: string }>,
	) {
		super(message);
		this.name = "RaceError";
	}
}

/**
 * Branch was cancelled (used by race).
 */
export class CancelledError extends Error {
	constructor() {
		super("Branch cancelled");
		this.name = "CancelledError";
	}
}

/**
 * Entry is currently being processed.
 * Thrown when user forgets to await a step.
 */
export class EntryInProgressError extends Error {
	constructor() {
		super(
			"Cannot start a new workflow entry while another is in progress. " +
				"Did you forget to await the previous step/loop/sleep?",
		);
		this.name = "EntryInProgressError";
	}
}
