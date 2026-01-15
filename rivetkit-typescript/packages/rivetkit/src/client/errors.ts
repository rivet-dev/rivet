export class ActorClientError extends Error {}

export class InternalError extends ActorClientError {}

export class ManagerError extends ActorClientError {
	constructor(error: string, opts?: ErrorOptions) {
		super(`Manager error: ${error}`, opts);
	}
}

export class MalformedResponseMessage extends ActorClientError {
	constructor(cause?: unknown) {
		super(`Malformed response message: ${cause}`, { cause });
	}
}

export class ActorError extends ActorClientError {
	__type = "ActorError";

	constructor(
		public readonly group: string,
		public readonly code: string,
		message: string,
		public readonly metadata?: unknown,
	) {
		super(message);
	}
}

export class HttpRequestError extends ActorClientError {
	constructor(message: string, opts?: { cause?: unknown }) {
		super(`HTTP request error: ${message}`, { cause: opts?.cause });
	}
}

export class ActorConnDisposed extends ActorClientError {
	constructor() {
		super("Attempting to interact with a disposed actor connection.");
	}
}

/**
 * Checks if an error code indicates a scheduling error that may have more details.
 */
export function isSchedulingError(group: string, code: string): boolean {
	return (
		group === "guard" &&
		(code === "actor_ready_timeout" || code === "actor_runner_failed")
	);
}

/**
 * Error thrown when actor scheduling fails.
 * Provides detailed information about why the actor failed to start.
 */
export class ActorSchedulingError extends ActorError {
	public readonly actorId: string;
	public readonly details: unknown;

	constructor(
		group: string,
		code: string,
		actorId: string,
		details: unknown,
	) {
		super(
			group,
			code,
			`Actor ${actorId} failed to start: ${JSON.stringify(details)}`,
			{ actorId, details },
		);
		this.name = "ActorSchedulingError";
		this.actorId = actorId;
		this.details = details;
	}
}
