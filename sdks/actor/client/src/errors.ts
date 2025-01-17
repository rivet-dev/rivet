import { MAX_CONN_PARAMS_SIZE } from "@rivet-gg/actor-common/network";

export class ActorClientError extends Error {}

export class InternalError extends ActorClientError {}

export class ManagerError extends ActorClientError {
	constructor(error: string, opts?: ErrorOptions) {
		super(`Manager error: ${error}`, opts);
	}
}

export class ConnectionParametersTooLong extends ActorClientError {
	constructor() {
		super(
			`Connection parameters must be less than ${MAX_CONN_PARAMS_SIZE} bytes`,
		);
	}
}

export class MalformedResponseMessage extends ActorClientError {
	constructor(cause?: unknown) {
		super(`Malformed response message: ${cause}`, { cause });
	}
}

export class RpcError extends ActorClientError {
	constructor(
		public readonly code: string,
		message: string,
		public readonly metadata?: unknown,
	) {
		super(message);
	}
}
