import { Duration, Predicate, Schema, SchemaGetter } from "effect";
import * as Rivetkit from "rivetkit";

const ReasonTypeId = "~@rivetkit/effect/RivetError/Reason" as const;
const TypeId = "~@rivetkit/effect/RivetError" as const;

// ============================================================================
// Reason classes
// ============================================================================
//
// One class per semantic infrastructure-failure category exposed by the
// engine and client. Each reason carries its own wire `group` and `code`
// (defaulted via `Schema.tag` so call sites don't pass them), plus an
// `isRetryable` getter so callers can match on the reason `_tag` and
// decide retry policy without hand-rolling group/code switches.
//
// User-defined errors thrown via `UserError` inside an actor action ride
// through on the action's declared `errorSchema` and arrive in the typed
// error channel directly. They only fall through to the generic
// `UnknownUserError` reason below when the action did not declare a
// matching schema.

/** `auth.forbidden` — `onAuth` rejected the request. */
export class Forbidden extends Schema.TaggedErrorClass<Forbidden>(
	`${ReasonTypeId}/Forbidden`,
)("Forbidden", {
	group: Schema.tag("auth"),
	code: Schema.tag("forbidden"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("not_found"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("stopping"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("restarting"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("action_not_found"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("action_timed_out"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("aborted"),
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
	group: Schema.tag("actor"),
	code: Schema.tag("overloaded"),
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

/**
 * `message.incoming_too_long` / `message.outgoing_too_long`. Match on
 * `code` to distinguish direction.
 */
export class MessageTooLong extends Schema.TaggedErrorClass<MessageTooLong>(
	`${ReasonTypeId}/MessageTooLong`,
)("MessageTooLong", {
	group: Schema.tag("message"),
	code: Schema.Literals(["incoming_too_long", "outgoing_too_long"]),
	message: Schema.String,
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
	group: Schema.tag("queue"),
	code: Schema.String,
	message: Schema.String,
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
	group: Schema.tag("encoding"),
	code: Schema.tag("invalid"),
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
	group: Schema.tag("request"),
	code: Schema.tag("invalid"),
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
	group: Schema.tag("client"),
	code: Schema.tag("connection_open_failed"),
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
	group: Schema.tag("client"),
	code: Schema.tag("get_params_failed"),
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
	group: Schema.tag("ws"),
	code: Schema.tag("going_away"),
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
	group: Schema.tag("guard"),
	code: Schema.String,
	message: Schema.String,
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
export class UnknownUserError extends Schema.TaggedErrorClass<UnknownUserError>(
	`${ReasonTypeId}/UnknownUserError`,
)("UnknownUserError", {
	group: Schema.tag("user"),
	code: Schema.String,
	message: Schema.String,
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
	group: Schema.tag("rivetkit"),
	code: Schema.tag("internal_error"),
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
	| UnknownUserError
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
		typeof UnknownUserError,
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
	UnknownUserError,
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

// Classification table: `"group.code"` → factory producing the matching
// reason from raw `(message, metadata)`. Reasons whose `code` is variable
// (queue, guard, user) and the rivetkit-core internal-error aliases are
// handled by the fallback below.
const fixedFactories: Record<
	string,
	(message: string, metadata: unknown) => Reason
> = {
	"auth.forbidden": (message) => new Forbidden({ message }),
	"actor.not_found": (message) => new ActorNotFound({ message }),
	"actor.stopping": (message) => new ActorStopping({ message }),
	"actor.restarting": (message, metadata) => {
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
	},
	"actor.action_not_found": (message) => new ActionNotFound({ message }),
	"actor.action_timed_out": (message) => new ActionTimedOut({ message }),
	"actor.aborted": (message) => new ActionAborted({ message }),
	"actor.overloaded": (message, metadata) => {
		const channel = readMetaField(metadata, "channel");
		const capacity = readMetaField(metadata, "capacity");
		const operation = readMetaField(metadata, "operation");
		return new Overloaded({
			message,
			...(typeof channel === "string" ? { channel } : {}),
			...(typeof capacity === "number" ? { capacity } : {}),
			...(typeof operation === "string" ? { operation } : {}),
		});
	},
	"message.incoming_too_long": (message) =>
		new MessageTooLong({ message, code: "incoming_too_long" }),
	"message.outgoing_too_long": (message) =>
		new MessageTooLong({ message, code: "outgoing_too_long" }),
	"encoding.invalid": (message) => new InvalidEncoding({ message }),
	"request.invalid": (message) => new InvalidRequest({ message }),
	"client.connection_open_failed": (message) =>
		new ConnectionOpenFailed({ message }),
	"client.get_params_failed": (message) => new GetParamsFailed({ message }),
	"ws.going_away": (message) => new ConnectionLost({ message }),
	"core.internal_error": (message) => new InternalError({ message }),
	"rivetkit.internal_error": (message) => new InternalError({ message }),
};

const reasonFromWire = ({
	group,
	code,
	message,
	metadata,
}: WirePayload): Reason => {
	const factory = fixedFactories[`${group}.${code}`];
	if (factory !== undefined) return factory(message, metadata);
	if (group === "queue") return new QueueError({ message, code });
	if (group === "guard") return new GuardError({ message, code });
	if (group === "user") {
		return new UnknownUserError({
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
};

// Per-reason metadata serialization. Reasons not listed have no
// metadata. `group`, `code`, and `message` are read straight off the
// instance, so this is the only mapping `reasonToWire` needs.
const metadataFromReason = (reason: Reason): unknown | undefined => {
	switch (reason._tag) {
		case "ActorRestarting": {
			const metadata: Record<string, unknown> = {};
			if (reason.retryAfter !== undefined) {
				metadata.retryAfterMs = Duration.toMillis(reason.retryAfter);
			}
			if (reason.phase !== undefined) metadata.phase = reason.phase;
			return Object.keys(metadata).length > 0 ? metadata : undefined;
		}
		case "Overloaded": {
			const metadata: Record<string, unknown> = {};
			if (reason.channel !== undefined) metadata.channel = reason.channel;
			if (reason.capacity !== undefined) metadata.capacity = reason.capacity;
			if (reason.operation !== undefined)
				metadata.operation = reason.operation;
			return Object.keys(metadata).length > 0 ? metadata : undefined;
		}
		case "UnknownUserError":
		case "UnknownError":
			return reason.metadata;
		default:
			return undefined;
	}
};

const reasonToWire = (reason: Reason): WirePayload => {
	const metadata = metadataFromReason(reason);
	return {
		group: reason.group,
		code: reason.code,
		message: reason.message,
		...(metadata !== undefined ? { metadata } : {}),
	};
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
