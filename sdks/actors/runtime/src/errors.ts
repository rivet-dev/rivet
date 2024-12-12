export const INTERNAL_ERROR_CODE = "internal_error";
export const INTERNAL_ERROR_DESCRIPTION =
	"Internal error. Read the actor logs for more details.";

export const USER_ERROR_CODE = "user_error";

interface ActorErrorOptions extends ErrorOptions {
	/** Error data can safely be serialized in a response to the client. */
	public?: boolean;
	/** Metadata assocaited with this error. This will be sent to clients. */
	metadata?: unknown;
}

export class ActorError extends Error {
	public public: boolean;
	public metadata?: unknown;

	constructor(
		public readonly code: string,
		message: string,
		opts?: ActorErrorOptions,
	) {
		super(message, { cause: opts?.cause });
		this.public = opts?.public ?? false;
		this.metadata = opts?.metadata;
	}
}

export class InternalError extends ActorError {
	constructor(message: string) {
		super(INTERNAL_ERROR_CODE, message);
	}
}

export class Unreachable extends InternalError {
	constructor(x: never) {
		super(`Unreachable case: ${x}`);
	}
}

export class StateNotEnabled extends ActorError {
	constructor() {
		super(
			"state_not_enabled",
			"State not enabled. Must implement `_onInitialize` to use state.",
		);
	}
}

export class ConnectionStateNotEnabled extends ActorError {
	constructor() {
		super(
			"connection_state_not_enabled",
			"Connection state not enabled. Must implement `_onBeforeConnect` to use connection state.",
		);
	}
}

export class RpcTimedOut extends ActorError {
	constructor() {
		super("rpc_timed_out", "RPC timed out.", { public: true });
	}
}

export class RpcNotFound extends ActorError {
	constructor() {
		super("rpc_not_found", "RPC not found.", { public: true });
	}
}

export class InvalidProtocolVersion extends ActorError {
	constructor(version?: string) {
		super(
			"invalid_protocol_version",
			`Invalid protocol version \`${version}\`.`,
			{ public: true },
		);
	}
}

export class InvalidProtocolFormat extends ActorError {
	constructor(format?: string) {
		super("invalid_protocol_format", `Invalid protocol format \`${format}\`.`, {
			public: true,
		});
	}
}

export class MalformedConnectionParameters extends ActorError {
	constructor(cause: unknown) {
		super(
			"malformed_connnection_parameters",
			`Malformed connection parameters: ${cause}`,
			{ public: true, cause },
		);
	}
}

export class MalformedMessage extends ActorError {
	constructor(cause?: unknown) {
		super("malformed_message", `Malformed message: ${cause}`, {
			public: true,
			cause,
		});
	}
}

export interface InvalidStateTypeOptions {
	path?: unknown;
}

export class InvalidStateType extends ActorError {
	constructor(opts?: InvalidStateTypeOptions) {
		let msg = "";
		if (opts?.path) {
			msg += `Attempted to set invalid state at path \`${opts.path}\`.`;
		} else {
			msg += "Attempted to set invalid state.";
		}
		msg += " State must be JSON serializable.";
		super("invalid_state_type", msg);
	}
}

export interface UserErrorOptions extends ErrorOptions {
	code?: string;
	meta: unknown;
}

/** Error that can be safely returned to the user. Usually used to indicate a user error. */
export class UserError extends ActorError {
	constructor(message: string, opts?: UserErrorOptions) {
		super(opts?.code ?? USER_ERROR_CODE, message);
	}
}
