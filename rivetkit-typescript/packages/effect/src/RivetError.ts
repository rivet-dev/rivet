import { Duration, Predicate, Schema, SchemaGetter } from "effect";
import * as Rivetkit from "rivetkit";

const ReasonTypeId = "~@rivetkit/effect/RivetError/Reason" as const;
const TypeId = "~@rivetkit/effect/RivetError" as const;

// ============================================================================
// Reason classes
// ============================================================================
//
// One class per semantic infrastructure-failure category exposed by the
// engine and client. Each reason is a `Schema.TaggedErrorClass` with an
// `isRetryable` getter so callers can match on the reason `_tag` (via
// `Effect.catchReason` / `Effect.catchReasons` / `Match`) and decide
// retry policy without hand-rolling group/code switches.
//
// User-defined errors (thrown via `UserError` inside an actor action) are
// the domain layer: they ride through on the action's declared
// `errorSchema` and arrive in the typed error channel directly. They
// only fall through to the generic `UserError` reason below when the
// action did not declare a matching schema.

/** `auth.forbidden` — `onAuth` rejected the request. */
export class Forbidden extends Schema.TaggedErrorClass<Forbidden>(
	`${ReasonTypeId}/Forbidden`,
)("Forbidden", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `actor.not_found` — gateway target resolution returned no actor. */
export class ActorNotFound extends Schema.TaggedErrorClass<ActorNotFound>(
	`${ReasonTypeId}/ActorNotFound`,
)("ActorNotFound", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `actor.stopping` — request arrived while actor is shutting down. */
export class ActorStopping extends Schema.TaggedErrorClass<ActorStopping>(
	`${ReasonTypeId}/ActorStopping`,
)("ActorStopping", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `actor.restarting` — actor mid-restart; retry after `retryAfter`. */
export class ActorRestarting extends Schema.TaggedErrorClass<ActorRestarting>(
	`${ReasonTypeId}/ActorRestarting`,
)("ActorRestarting", {
	message: Schema.String,
	retryAfter: Schema.optional(Schema.Duration),
	phase: Schema.optional(
		Schema.Literals(["stopping", "sleeping", "waking", "runner_shutdown"]),
	),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return true;
	}
}

/** `actor.action_not_found` — no action by that name on the actor. */
export class ActionNotFound extends Schema.TaggedErrorClass<ActionNotFound>(
	`${ReasonTypeId}/ActionNotFound`,
)("ActionNotFound", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `actor.action_timed_out` — server-side action timeout. */
export class ActionTimedOut extends Schema.TaggedErrorClass<ActionTimedOut>(
	`${ReasonTypeId}/ActionTimedOut`,
)("ActionTimedOut", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return true;
	}
}

/** `actor.aborted` — action explicitly aborted server-side. */
export class ActionAborted extends Schema.TaggedErrorClass<ActionAborted>(
	`${ReasonTypeId}/ActionAborted`,
)("ActionAborted", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `actor.overloaded` — actor channel at capacity. */
export class Overloaded extends Schema.TaggedErrorClass<Overloaded>(
	`${ReasonTypeId}/Overloaded`,
)("Overloaded", {
	message: Schema.String,
	channel: Schema.optional(Schema.String),
	capacity: Schema.optional(Schema.Number),
	operation: Schema.optional(Schema.String),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return true;
	}
}

/** `message.incoming_too_long` / `message.outgoing_too_long`. */
export class MessageTooLong extends Schema.TaggedErrorClass<MessageTooLong>(
	`${ReasonTypeId}/MessageTooLong`,
)("MessageTooLong", {
	message: Schema.String,
	direction: Schema.Literals(["incoming", "outgoing"]),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

const queueRetryableCodes = new Set<string>(["full", "timed_out"]);

/** `queue.*` — queue-related server errors. `code` keeps the raw engine code. */
export class QueueError extends Schema.TaggedErrorClass<QueueError>(
	`${ReasonTypeId}/QueueError`,
)("QueueError", {
	message: Schema.String,
	code: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return queueRetryableCodes.has(this.code);
	}
}

/** `encoding.invalid` — unsupported encoding negotiated. */
export class InvalidEncoding extends Schema.TaggedErrorClass<InvalidEncoding>(
	`${ReasonTypeId}/InvalidEncoding`,
)("InvalidEncoding", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `request.invalid` — malformed `ActorQuery` or request payload. */
export class InvalidRequest extends Schema.TaggedErrorClass<InvalidRequest>(
	`${ReasonTypeId}/InvalidRequest`,
)("InvalidRequest", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `client.connection_open_failed` — websocket open failed. */
export class ConnectionOpenFailed extends Schema.TaggedErrorClass<ConnectionOpenFailed>(
	`${ReasonTypeId}/ConnectionOpenFailed`,
)("ConnectionOpenFailed", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return true;
	}
}

/** `client.get_params_failed` — user `getParams()` callback threw. */
export class GetParamsFailed extends Schema.TaggedErrorClass<GetParamsFailed>(
	`${ReasonTypeId}/GetParamsFailed`,
)("GetParamsFailed", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/** `ws.going_away` / generic transport drop — connection lost, retry. */
export class ConnectionLost extends Schema.TaggedErrorClass<ConnectionLost>(
	`${ReasonTypeId}/ConnectionLost`,
)("ConnectionLost", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return true;
	}
}

const guardRetryableCodes = new Set<string>([
	"actor_runner_failed",
	"actor_ready_timeout",
	"service_unavailable",
]);

/** `guard.*` — engine guard/scheduler failures; `code` keeps the raw engine code. */
export class GuardError extends Schema.TaggedErrorClass<GuardError>(
	`${ReasonTypeId}/GuardError`,
)("GuardError", {
	message: Schema.String,
	code: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return guardRetryableCodes.has(this.code);
	}
}

/**
 * Open-ended user error reason. Used when the actor threw `UserError` but
 * the failing action did not declare a matching schema in its `error`
 * field — so we can't surface it as a typed domain error in the Effect
 * error channel.
 *
 * Actions that declare their user errors via `Action.make({ error: ... })`
 * receive those errors **typed** in the error channel; this reason is
 * the catch-all for everything else.
 */
export class UserError extends Schema.TaggedErrorClass<UserError>(
	`${ReasonTypeId}/UserError`,
)("UserError", {
	message: Schema.String,
	code: Schema.String,
	metadata: Schema.optional(Schema.Unknown),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/**
 * Sanitized internal error from `rivetkit-core`. Original details live in
 * the server logs (or set `RIVET_EXPOSE_ERRORS=1` to inline them in dev).
 */
export class InternalError extends Schema.TaggedErrorClass<InternalError>(
	`${ReasonTypeId}/InternalError`,
)("InternalError", {
	message: Schema.String,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

/**
 * Forward-compatible catch-all for `(group, code)` pairs the SDK does
 * not recognize yet. Keeps the raw wire fields so newer engine errors
 * still surface usefully through older SDKs.
 */
export class UnknownError extends Schema.TaggedErrorClass<UnknownError>(
	`${ReasonTypeId}/UnknownError`,
)("UnknownError", {
	group: Schema.String,
	code: Schema.String,
	message: Schema.String,
	metadata: Schema.optional(Schema.Unknown),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

// ============================================================================
// Reason union
// ============================================================================

export type Reason =
	| Forbidden
	| ActorNotFound
	| ActorStopping
	| ActorRestarting
	| ActionNotFound
	| ActionTimedOut
	| ActionAborted
	| Overloaded
	| MessageTooLong
	| QueueError
	| InvalidEncoding
	| InvalidRequest
	| ConnectionOpenFailed
	| GetParamsFailed
	| ConnectionLost
	| GuardError
	| UserError
	| InternalError
	| UnknownError;

export const Reason: Schema.Union<
	[
		typeof Forbidden,
		typeof ActorNotFound,
		typeof ActorStopping,
		typeof ActorRestarting,
		typeof ActionNotFound,
		typeof ActionTimedOut,
		typeof ActionAborted,
		typeof Overloaded,
		typeof MessageTooLong,
		typeof QueueError,
		typeof InvalidEncoding,
		typeof InvalidRequest,
		typeof ConnectionOpenFailed,
		typeof GetParamsFailed,
		typeof ConnectionLost,
		typeof GuardError,
		typeof UserError,
		typeof InternalError,
		typeof UnknownError,
	]
> = Schema.Union([
	Forbidden,
	ActorNotFound,
	ActorStopping,
	ActorRestarting,
	ActionNotFound,
	ActionTimedOut,
	ActionAborted,
	Overloaded,
	MessageTooLong,
	QueueError,
	InvalidEncoding,
	InvalidRequest,
	ConnectionOpenFailed,
	GetParamsFailed,
	ConnectionLost,
	GuardError,
	UserError,
	InternalError,
	UnknownError,
]);

export const isReason = (u: unknown): u is Reason =>
	Predicate.hasProperty(u, ReasonTypeId);

// ============================================================================
// Top-level RivetError
// ============================================================================

/**
 * The infrastructure-failure error surfaced by `@rivetkit/effect` action
 * calls. Wraps a discriminated `reason` of all known engine and client
 * failure modes.
 *
 * Recover with `Effect.catchReason` / `Effect.catchReasons` /
 * `Effect.unwrapReason`:
 *
 * ```ts
 * program.pipe(
 *   Effect.catchReason("RivetError", "ActorRestarting", (r) =>
 *     Effect.sleep(r.retryAfter ?? "100 millis").pipe(Effect.andThen(retry)),
 *   ),
 *   Effect.catchReasons("RivetError", {
 *     Forbidden: () => Effect.fail(new MyAuthError()),
 *     ConnectionLost: () => Effect.logWarning("reconnecting"),
 *   }),
 * )
 * ```
 *
 * User-defined errors declared on an action via `Action.make({ error })`
 * arrive in the typed error channel separately and do NOT flow through
 * `RivetError`.
 */
export class RivetError extends Schema.TaggedErrorClass<RivetError>(
	"@rivetkit/effect/RivetError",
)("RivetError", {
	reason: Reason,
}) {
	readonly [TypeId] = TypeId;
	override readonly cause = this.reason;

	get isRetryable(): boolean {
		return this.reason.isRetryable;
	}

	get retryAfter(): Duration.Duration | undefined {
		return "retryAfter" in this.reason
			? (this.reason.retryAfter as Duration.Duration | undefined)
			: undefined;
	}

	override get message(): string {
		return this.reason.message || this.reason._tag;
	}
}

export const isRivetError = (u: unknown): u is RivetError =>
	Predicate.hasProperty(u, TypeId);

// ============================================================================
// Wire codec
// ============================================================================
//
// On-the-wire envelope produced by `rivetkit-core`'s defect sanitizer.
// `Pick`ing here anchors the codec against drift in the canonical wire
// shape.

type WirePayload = Pick<
	Rivetkit.RivetErrorLike,
	"group" | "code" | "message" | "metadata"
>;

const Wire = Schema.Struct({
	group: Schema.String,
	code: Schema.String,
	message: Schema.String,
	metadata: Schema.optionalKey(Schema.Unknown),
});

const readMetaField = (metadata: unknown, key: string): unknown => {
	if (typeof metadata !== "object" || metadata === null) return undefined;
	return (metadata as Record<string, unknown>)[key];
};

const reasonFromWire = ({
	group,
	code,
	message,
	metadata,
}: WirePayload): Reason => {
	switch (`${group}.${code}`) {
		case "auth.forbidden":
			return new Forbidden({ message });
		case "actor.not_found":
			return new ActorNotFound({ message });
		case "actor.stopping":
			return new ActorStopping({ message });
		case "actor.restarting": {
			const retryAfterMs = readMetaField(metadata, "retryAfterMs");
			const phase = readMetaField(metadata, "phase");
			const allowedPhases = new Set<string>([
				"stopping",
				"sleeping",
				"waking",
				"runner_shutdown",
			]);
			return new ActorRestarting({
				message,
				...(typeof retryAfterMs === "number"
					? { retryAfter: Duration.millis(retryAfterMs) }
					: {}),
				...(typeof phase === "string" && allowedPhases.has(phase)
					? { phase: phase as ActorRestarting["phase"] }
					: {}),
			});
		}
		case "actor.action_not_found":
			return new ActionNotFound({ message });
		case "actor.action_timed_out":
			return new ActionTimedOut({ message });
		case "actor.aborted":
			return new ActionAborted({ message });
		case "actor.overloaded": {
			const channel = readMetaField(metadata, "channel");
			const capacity = readMetaField(metadata, "capacity");
			const operation = readMetaField(metadata, "operation");
			return new Overloaded({
				message,
				...(typeof channel === "string" ? { channel } : {}),
				...(typeof capacity === "number" ? { capacity } : {}),
				...(typeof operation === "string" ? { operation } : {}),
			});
		}
		case "message.incoming_too_long":
			return new MessageTooLong({ message, direction: "incoming" });
		case "message.outgoing_too_long":
			return new MessageTooLong({ message, direction: "outgoing" });
		case "encoding.invalid":
			return new InvalidEncoding({ message });
		case "request.invalid":
			return new InvalidRequest({ message });
		case "client.connection_open_failed":
			return new ConnectionOpenFailed({ message });
		case "client.get_params_failed":
			return new GetParamsFailed({ message });
		case "ws.going_away":
			return new ConnectionLost({ message });
		case "core.internal_error":
		case "rivetkit.internal_error":
			return new InternalError({ message });
		default:
			if (group === "queue") return new QueueError({ message, code });
			if (group === "guard") return new GuardError({ message, code });
			if (group === "user") {
				return new UserError({
					message,
					code,
					...(metadata !== undefined ? { metadata } : {}),
				});
			}
			return new UnknownError({
				group,
				code,
				message,
				...(metadata !== undefined ? { metadata } : {}),
			});
	}
};

const reasonToWire = (reason: Reason): WirePayload => {
	switch (reason._tag) {
		case "Forbidden":
			return {
				group: "auth",
				code: "forbidden",
				message: reason.message,
			};
		case "ActorNotFound":
			return {
				group: "actor",
				code: "not_found",
				message: reason.message,
			};
		case "ActorStopping":
			return {
				group: "actor",
				code: "stopping",
				message: reason.message,
			};
		case "ActorRestarting": {
			const metadata: Record<string, unknown> = {};
			if (reason.retryAfter !== undefined) {
				metadata.retryAfterMs = Duration.toMillis(reason.retryAfter);
			}
			if (reason.phase !== undefined) metadata.phase = reason.phase;
			return {
				group: "actor",
				code: "restarting",
				message: reason.message,
				...(Object.keys(metadata).length > 0 ? { metadata } : {}),
			};
		}
		case "ActionNotFound":
			return {
				group: "actor",
				code: "action_not_found",
				message: reason.message,
			};
		case "ActionTimedOut":
			return {
				group: "actor",
				code: "action_timed_out",
				message: reason.message,
			};
		case "ActionAborted":
			return { group: "actor", code: "aborted", message: reason.message };
		case "Overloaded": {
			const metadata: Record<string, unknown> = {};
			if (reason.channel !== undefined) metadata.channel = reason.channel;
			if (reason.capacity !== undefined)
				metadata.capacity = reason.capacity;
			if (reason.operation !== undefined)
				metadata.operation = reason.operation;
			return {
				group: "actor",
				code: "overloaded",
				message: reason.message,
				...(Object.keys(metadata).length > 0 ? { metadata } : {}),
			};
		}
		case "MessageTooLong":
			return {
				group: "message",
				code:
					reason.direction === "incoming"
						? "incoming_too_long"
						: "outgoing_too_long",
				message: reason.message,
			};
		case "QueueError":
			return {
				group: "queue",
				code: reason.code,
				message: reason.message,
			};
		case "InvalidEncoding":
			return {
				group: "encoding",
				code: "invalid",
				message: reason.message,
			};
		case "InvalidRequest":
			return {
				group: "request",
				code: "invalid",
				message: reason.message,
			};
		case "ConnectionOpenFailed":
			return {
				group: "client",
				code: "connection_open_failed",
				message: reason.message,
			};
		case "GetParamsFailed":
			return {
				group: "client",
				code: "get_params_failed",
				message: reason.message,
			};
		case "ConnectionLost":
			return { group: "ws", code: "going_away", message: reason.message };
		case "GuardError":
			return {
				group: "guard",
				code: reason.code,
				message: reason.message,
			};
		case "UserError":
			return {
				group: "user",
				code: reason.code,
				message: reason.message,
				...(reason.metadata !== undefined
					? { metadata: reason.metadata }
					: {}),
			};
		case "InternalError":
			return {
				group: "rivetkit",
				code: "internal_error",
				message: reason.message,
			};
		case "UnknownError":
			return {
				group: reason.group,
				code: reason.code,
				message: reason.message,
				...(reason.metadata !== undefined
					? { metadata: reason.metadata }
					: {}),
			};
	}
};

/**
 * Wire codec used as the default `defectSchema` for actions. Decodes
 * the `(group, code, message, metadata)` envelope produced by
 * `rivetkit-core`'s defect sanitizer into a `RivetError` carrying the
 * appropriate semantic `reason`.
 */
export const RivetErrorFromWire = Wire.pipe(
	Schema.decodeTo(Schema.instanceOf(RivetError), {
		decode: SchemaGetter.transform(
			(wire) => new RivetError({ reason: reasonFromWire(wire) }),
		),
		encode: SchemaGetter.transform((e: RivetError) =>
			reasonToWire(e.reason),
		),
	}),
);

export const decodeRivetErrorFromWire =
	Schema.decodeUnknownEffect(RivetErrorFromWire);
