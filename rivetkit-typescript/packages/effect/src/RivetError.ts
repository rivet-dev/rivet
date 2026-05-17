import { Duration, Option, Predicate, Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";
import * as RivetkitRivetError from "./internal/RivetRivetError";

const ReasonTypeId = "~@rivetkit/effect/RivetError/Reason" as const;
const TypeId = "~@rivetkit/effect/RivetError" as const;

export class Forbidden extends Schema.TaggedErrorClass<Forbidden>(
	`${ReasonTypeId}/Forbidden`,
)("Forbidden", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ActorNotFound extends Schema.TaggedErrorClass<ActorNotFound>(
	`${ReasonTypeId}/ActorNotFound`,
)("ActorNotFound", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ActorStopping extends Schema.TaggedErrorClass<ActorStopping>(
	`${ReasonTypeId}/ActorStopping`,
)("ActorStopping", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ActorRestarting extends Schema.TaggedErrorClass<ActorRestarting>(
	`${ReasonTypeId}/ActorRestarting`,
)("ActorRestarting", {
	retryAfter: Schema.optional(Schema.Duration),
	phase: Schema.optional(
		Schema.Literals(["stopping", "sleeping", "waking", "runner_shutdown"]),
	),
	cause: RivetkitRivetError.RivetkitRivetError,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return true;
	}
}

export class ActionNotFound extends Schema.TaggedErrorClass<ActionNotFound>(
	`${ReasonTypeId}/ActionNotFound`,
)("ActionNotFound", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ActionTimedOut extends Schema.TaggedErrorClass<ActionTimedOut>(
	`${ReasonTypeId}/ActionTimedOut`,
)("ActionTimedOut", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return true;
	}
}

export class ActionAborted extends Schema.TaggedErrorClass<ActionAborted>(
	`${ReasonTypeId}/ActionAborted`,
)("ActionAborted", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ActorOverloaded extends Schema.TaggedErrorClass<ActorOverloaded>(
	`${ReasonTypeId}/ActorOverloaded`,
)("ActorOverloaded", {
	channel: Schema.optional(Schema.String),
	capacity: Schema.optional(Schema.Number),
	operation: Schema.optional(Schema.String),
	cause: RivetkitRivetError.RivetkitRivetError,
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return true;
	}
}

export class MessageTooLong extends Schema.TaggedErrorClass<MessageTooLong>(
	`${ReasonTypeId}/MessageTooLong`,
)("MessageTooLong", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class InvalidEncoding extends Schema.TaggedErrorClass<InvalidEncoding>(
	`${ReasonTypeId}/InvalidEncoding`,
)("InvalidEncoding", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class InvalidRequest extends Schema.TaggedErrorClass<InvalidRequest>(
	`${ReasonTypeId}/InvalidRequest`,
)("InvalidRequest", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ConnectionOpenFailed extends Schema.TaggedErrorClass<ConnectionOpenFailed>(
	`${ReasonTypeId}/ConnectionOpenFailed`,
)("ConnectionOpenFailed", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return true;
	}
}

export class GetParamsFailed extends Schema.TaggedErrorClass<GetParamsFailed>(
	`${ReasonTypeId}/GetParamsFailed`,
)("GetParamsFailed", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

export class ConnectionLost extends Schema.TaggedErrorClass<ConnectionLost>(
	`${ReasonTypeId}/ConnectionLost`,
)("ConnectionLost", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return true;
	}
}

const guardRetryableCodes = new Set<string>([
	"actor_runner_failed",
	"actor_ready_timeout",
	"service_unavailable",
]);

export class GuardError extends Schema.TaggedErrorClass<GuardError>(
	`${ReasonTypeId}/GuardError`,
)("GuardError", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return guardRetryableCodes.has(this.cause.code);
	}
}

export class InternalError extends Schema.TaggedErrorClass<InternalError>(
	`${ReasonTypeId}/InternalError`,
)("InternalError", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
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
)("UnknownUserError", { cause: RivetkitRivetError.RivetkitRivetError }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message(): string {
		return this.cause.message;
	}
	get isRetryable(): boolean {
		return false;
	}
}

/**
 * Forward-compatible catch-all for `(group, code)` pairs the SDK does
 * not recognize yet, and for malformed non-Rivet failures. Known wire
 * fields are mirrored when present, while `cause` preserves the raw input.
 */
export class UnknownError extends Schema.TaggedErrorClass<UnknownError>(
	`${ReasonTypeId}/UnknownError`,
)("UnknownError", {
	message: Schema.String,
	cause: Schema.Union([RivetkitRivetError.RivetkitRivetError, Schema.Defect]),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get isRetryable(): boolean {
		return false;
	}
}

export type Reason =
	| Forbidden
	| ActorNotFound
	| ActorStopping
	| ActorRestarting
	| ActionNotFound
	| ActionTimedOut
	| ActionAborted
	| ActorOverloaded
	| MessageTooLong
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
		typeof ActorOverloaded,
		typeof MessageTooLong,
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
	ActorOverloaded,
	MessageTooLong,
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

/**
 * The infrastructure-failure error surfaced by `@rivetkit/effect`
 * calls. Wraps a discriminated `reason` of all known failure
 * modes.
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

const readMetaField = (metadata: unknown, key: string): unknown => {
	if (typeof metadata !== "object" || metadata === null) return undefined;
	return (metadata as Record<string, unknown>)[key];
};

const simpleReasonByCode: Record<
	string,
	new (props: {
		cause: RivetkitRivetError.RivetkitRivetError;
	}) => Reason
> = {
	"auth.forbidden": Forbidden,
	"actor.not_found": ActorNotFound,
	"actor.stopping": ActorStopping,
	"actor.action_not_found": ActionNotFound,
	"actor.action_timed_out": ActionTimedOut,
	"actor.aborted": ActionAborted,
	"actor.overloaded": ActorOverloaded,
	"message.incoming_too_long": MessageTooLong,
	"message.outgoing_too_long": MessageTooLong,
	"encoding.invalid": InvalidEncoding,
	"request.invalid": InvalidRequest,
	"client.connection_open_failed": ConnectionOpenFailed,
	"client.get_params_failed": GetParamsFailed,
	"ws.going_away": ConnectionLost,
	"core.internal_error": InternalError,
	"rivetkit.internal_error": InternalError,
};

// Static check that every key above is a canonical (group, code) pair
// recognized by `rivetkit/errors`. Renaming a wire code on the canonical
// side surfaces here as a runtime failure during module init.
for (const key of Object.keys(simpleReasonByCode)) {
	const [group, code] = key.split(".") as [string, string];
	if (
		!RivetkitErrors.isRivetErrorCode(
			{ group, code, message: "" },
			group,
			code,
		)
	) {
		throw new Error(`unknown rivetkit error code: ${key}`);
	}
}

const allowedRestartingPhases = new Set<string>([
	"stopping",
	"sleeping",
	"waking",
	"runner_shutdown",
]);

const reasonFromRivetkitRivetError = (
	error: RivetkitRivetError.RivetkitRivetError,
): Reason => {
	const Cls = simpleReasonByCode[`${error.group}.${error.code}`];
	if (Cls) return new Cls({ cause: error });
	if (error.group === "actor" && error.code === "restarting") {
		const retryAfterMs = readMetaField(error.metadata, "retryAfterMs");
		const phase = readMetaField(error.metadata, "phase");
		return new ActorRestarting({
			cause: error,
			...(typeof retryAfterMs === "number"
				? { retryAfter: Duration.millis(retryAfterMs) }
				: {}),
			...(typeof phase === "string" && allowedRestartingPhases.has(phase)
				? { phase: phase as ActorRestarting["phase"] }
				: {}),
		});
	}
	if (error.group === "guard") return new GuardError({ cause: error });
	if (error.group === "user") return new UnknownUserError({ cause: error });
	return new UnknownError({
		message: error.message,
		cause: error,
	});
};

export const fromRivetkitRivetError = (
	e: RivetkitRivetError.RivetkitRivetError,
): RivetError => new RivetError({ reason: reasonFromRivetkitRivetError(e) });

const decodeRivetkitRivetErrorOption = Schema.decodeUnknownOption(
	RivetkitRivetError.RivetkitRivetError,
);

export const fromUnknown = (cause: unknown): RivetError => {
	if (isRivetError(cause)) return cause;

	const decoded = decodeRivetkitRivetErrorOption(cause);
	if (Option.isSome(decoded)) return fromRivetkitRivetError(decoded.value);

	return new RivetError({
		reason: new UnknownError({
			message: "Unknown error",
			cause,
		}),
	});
};
