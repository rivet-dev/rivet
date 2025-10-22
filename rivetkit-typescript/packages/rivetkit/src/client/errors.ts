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
