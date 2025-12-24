import { MAX_CONN_PARAMS_SIZE } from "@/common//network";

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

// === Actor Scheduling Error Types ===

/**
 * Errors from serverless connection workflow.
 * Matches ServerlessConnectionError from API.
 */
export type ServerlessConnectionError =
	| { http_error: { status_code: number; body: string } }
	| "stream_ended_early"
	| { connection_error: { message: string } }
	| "invalid_base64"
	| { invalid_payload: { message: string } }
	| "runner_config_not_found"
	| "runner_config_not_serverless"
	| "namespace_not_found";

/**
 * Actor error details from API response.
 * Matches ActorError from API.
 */
export type ActorErrorDetails =
	| { serverless_error: ServerlessConnectionError }
	| { no_capacity: { runner_name: string } }
	| { runner_no_response: { runner_id: string } };

/**
 * Error thrown when actor scheduling fails.
 * Provides detailed information about why the actor failed to start.
 */
export class ActorSchedulingError extends ActorClientError {
	public readonly actorId: string;
	public readonly errorType: string;
	public readonly details: ActorErrorDetails;

	constructor(actorId: string, error: ActorErrorDetails) {
		const message = ActorSchedulingError.formatMessage(error);
		super(message);
		this.name = "ActorSchedulingError";
		this.actorId = actorId;
		this.errorType = Object.keys(error)[0];
		this.details = error;
	}

	static formatMessage(error: ActorErrorDetails): string {
		if ("serverless_error" in error) {
			const se = error.serverless_error;
			if (typeof se === "string") {
				return `Serverless error: ${se.replace(/_/g, " ")}`;
			}
			if ("http_error" in se) {
				return `Serverless HTTP ${se.http_error.status_code}: ${se.http_error.body}`;
			}
			if ("connection_error" in se) {
				return `Serverless connection error: ${se.connection_error.message}`;
			}
			if ("invalid_payload" in se) {
				return `Invalid serverless payload: ${se.invalid_payload.message}`;
			}
			return "Unknown serverless error";
		}
		if ("no_capacity" in error) {
			return `No capacity available for runner: ${error.no_capacity.runner_name}`;
		}
		if ("runner_no_response" in error) {
			return `Runner ${error.runner_no_response.runner_id} did not respond`;
		}
		return "Unknown scheduling error";
	}
}
