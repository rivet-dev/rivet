import type { DeconstructedError } from "@/common/utils";

export const INTERNAL_ERROR_CODE = "internal_error";
export const INTERNAL_ERROR_DESCRIPTION = "An internal error occurred";
export type InternalErrorMetadata = Record<string, never>;

export const USER_ERROR_CODE = "user_error";
export const BRIDGE_RIVET_ERROR_PREFIX = "__RIVET_ERROR_JSON__:";

export interface RivetErrorOptions extends ErrorOptions {
	/** Error data can safely be serialized in a response to the client. */
	public?: boolean;
	/** Metadata associated with this error. */
	metadata?: unknown;
	/** Explicit HTTP status override for router responses. */
	statusCode?: number;
}

export interface RivetErrorLike {
	__type?: "ActorError" | "RivetError";
	group: string;
	code: string;
	message: string;
	metadata?: unknown;
	public?: boolean;
	statusCode?: number;
}

export interface UserErrorOptions extends ErrorOptions {
	/**
	 * Machine readable code for this error. Useful for catching different types of
	 * errors in try-catch.
	 */
	code?: string;
	/**
	 * Additional metadata related to the error. Useful for understanding context
	 * about the error.
	 */
	metadata?: unknown;
}

function looksLikeRivetErrorOptions(
	value: unknown,
): value is RivetErrorOptions {
	return (
		typeof value === "object" &&
		value !== null &&
		("public" in value ||
			"metadata" in value ||
			"statusCode" in value ||
			"cause" in value)
	);
}

function isTypedErrorTag(value: unknown): value is "ActorError" | "RivetError" {
	return value === "ActorError" || value === "RivetError";
}

function errorMessage(error: unknown, fallback = String(error)): string {
	if (
		error &&
		typeof error === "object" &&
		"message" in error &&
		typeof error.message === "string"
	) {
		return error.message;
	}

	return fallback;
}

function normalizeDecodedBridgePayload(
	payload: RivetErrorLike,
): RivetErrorLike {
	if (payload.public !== undefined || payload.statusCode !== undefined) {
		return payload;
	}

	if (payload.group === "auth" && payload.code === "forbidden") {
		return {
			...payload,
			public: true,
			statusCode: 403,
		};
	}

	if (payload.group === "actor" && payload.code === "action_not_found") {
		return {
			...payload,
			public: true,
			statusCode: 404,
		};
	}

	if (payload.group === "actor" && payload.code === "action_timed_out") {
		return {
			...payload,
			public: true,
			statusCode: 408,
		};
	}

	if (payload.group === "actor" && payload.code === "aborted") {
		return {
			...payload,
			public: true,
			statusCode: 400,
		};
	}

	if (
		payload.group === "message" &&
		(payload.code === "incoming_too_long" ||
			payload.code === "outgoing_too_long")
	) {
		return {
			...payload,
			public: true,
			statusCode: 400,
		};
	}

	if (
		payload.group === "queue" &&
		[
			"full",
			"message_too_large",
			"message_invalid",
			"invalid_payload",
			"invalid_completion_payload",
			"already_completed",
			"previous_message_not_completed",
			"complete_not_configured",
			"timed_out",
		].includes(payload.code)
	) {
		return {
			...payload,
			public: true,
			statusCode: 400,
		};
	}

	return payload;
}

export function isRivetErrorLike(
	error: unknown,
): error is RivetError | DeconstructedError | RivetErrorLike {
	return (
		typeof error === "object" &&
		error !== null &&
		"group" in error &&
		typeof error.group === "string" &&
		"code" in error &&
		typeof error.code === "string" &&
		"message" in error &&
		typeof error.message === "string" &&
		(!("__type" in error) || isTypedErrorTag(error.__type))
	);
}

export class RivetError extends Error {
	__type = "RivetError" as const;

	public public: boolean;
	public metadata?: unknown;
	public statusCode: number;
	public readonly group: string;
	public readonly code: string;

	public static isRivetError(
		error: unknown,
	): error is RivetError | DeconstructedError {
		return isRivetErrorLike(error);
	}

	public static isActorError(
		error: unknown,
	): error is RivetError | DeconstructedError {
		return isRivetErrorLike(error);
	}

	constructor(
		group: string,
		code: string,
		message: string,
		options?: RivetErrorOptions | unknown,
	) {
		const normalized = looksLikeRivetErrorOptions(options)
			? options
			: { metadata: options };

		super(message, { cause: normalized.cause });
		this.name = "RivetError";
		this.group = group;
		this.code = code;
		this.public = normalized.public ?? false;
		this.metadata = normalized.metadata;
		this.statusCode =
			normalized.statusCode ?? (this.public ? 400 : 500);
	}

	toString() {
		return this.message;
	}
}

export { RivetError as ActorError };

export class UserError extends RivetError {
	constructor(message: string, options?: UserErrorOptions) {
		super("user", options?.code ?? USER_ERROR_CODE, message, {
			public: true,
			metadata: options?.metadata,
			cause: options?.cause,
		});
	}
}

export function toRivetError(
	error: unknown,
	fallback?: Partial<RivetErrorLike>,
): RivetError {
	if (typeof error === "string") {
		const bridged = decodeBridgeRivetError(error);
		if (bridged) {
			return bridged;
		}
	}

	if (error instanceof Error) {
		const bridged = decodeBridgeRivetError(error.message);
		if (bridged) {
			return bridged;
		}
	}

	if (isRivetErrorLike(error)) {
		return new RivetError(error.group, error.code, error.message, {
			public: error.public,
			statusCode: error.statusCode,
			metadata: error.metadata,
			cause: error instanceof Error ? error.cause : undefined,
		});
	}

	return new RivetError(
		fallback?.group ?? "actor",
		fallback?.code ?? INTERNAL_ERROR_CODE,
		errorMessage(error, fallback?.message ?? "Unknown error"),
		{
			public: fallback?.public,
			statusCode: fallback?.statusCode,
			metadata: fallback?.metadata,
			cause: error instanceof Error ? error : undefined,
		},
	);
}

export function encodeBridgeRivetError(error: RivetErrorLike): string {
	return `${BRIDGE_RIVET_ERROR_PREFIX}${JSON.stringify({
		group: error.group,
		code: error.code,
		message: error.message,
		metadata: error.metadata,
		public: error.public,
		statusCode: error.statusCode,
	})}`;
}

export function decodeBridgeRivetError(
	value: string,
): RivetError | undefined {
	if (!value.startsWith(BRIDGE_RIVET_ERROR_PREFIX)) {
		return undefined;
	}

	try {
		const payload = normalizeDecodedBridgePayload(
			JSON.parse(
			value.slice(BRIDGE_RIVET_ERROR_PREFIX.length),
			) as RivetErrorLike,
		);
		if (!isRivetErrorLike(payload)) {
			return undefined;
		}

		return new RivetError(payload.group, payload.code, payload.message, {
			metadata: payload.metadata,
			public: payload.public,
			statusCode: payload.statusCode,
		});
	} catch {
		return undefined;
	}
}

export function isRivetErrorCode(
	error: unknown,
	group: string,
	code: string,
): error is RivetError {
	return isRivetErrorLike(error) && error.group === group && error.code === code;
}

export function internalError(
	message: string,
	options?: Partial<RivetErrorOptions> & {
		group?: string;
		code?: string;
	},
): RivetError {
	return new RivetError(
		options?.group ?? "actor",
		options?.code ?? INTERNAL_ERROR_CODE,
		message,
		{
			public: options?.public,
			statusCode: options?.statusCode,
			metadata: options?.metadata,
			cause: options?.cause,
		},
	);
}

export function invalidEncoding(format?: string): RivetError {
	return new RivetError(
		"encoding",
		"invalid",
		`Invalid encoding \`${format}\`. (https://www.rivet.dev/docs/clients/javascript)`,
		{
			public: true,
		},
	);
}

export function invalidRequest(error?: unknown): RivetError {
	return new RivetError(
		"request",
		"invalid",
		`Invalid request: ${errorMessage(error, String(error))}`,
		{
			public: true,
			cause: error instanceof Error ? error : undefined,
		},
	);
}

export function actorNotFound(identifier?: string): RivetError {
	return new RivetError(
		"actor",
		"not_found",
		identifier
			? `Actor not found: ${identifier} (https://www.rivet.dev/docs/clients/javascript)`
			: "Actor not found (https://www.rivet.dev/docs/clients/javascript)",
		{ public: true },
	);
}

export function actorStopping(identifier?: string): RivetError {
	return new RivetError(
		"actor",
		"stopping",
		identifier ? `Actor stopping: ${identifier}` : "Actor stopping",
		{ public: true },
	);
}

export interface ActorRestartingOptions {
	phase?: "stopping" | "sleeping" | "waking" | "runner_shutdown";
	retryAfterMs?: number;
}

export function actorRestarting(opts?: ActorRestartingOptions): RivetError {
	return new RivetError(
		"actor",
		"restarting",
		"Actor is restarting. Retry the request.",
		{
			public: true,
			statusCode: 503,
			metadata: {
				retryable: true,
				...(opts?.phase ? { phase: opts.phase } : {}),
				...(opts?.retryAfterMs !== undefined
					? { retryAfterMs: opts.retryAfterMs }
					: {}),
			},
		},
	);
}

export function forbiddenError(): RivetError {
	return new RivetError("auth", "forbidden", "Forbidden", {
		public: true,
		statusCode: 403,
	});
}

export function unsupportedFeature(feature: string): RivetError {
	return new RivetError(
		"feature",
		"unsupported",
		`Unsupported feature: ${feature}`,
	);
}
