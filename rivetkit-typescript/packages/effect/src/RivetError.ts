import { Predicate, Schema } from "effect";
import * as RivetkitErrors from "rivetkit/errors";

const ReasonTypeId = "~@rivetkit/effect/RivetError/Reason" as const;
const TypeId = "~@rivetkit/effect/RivetError" as const;

export class Forbidden extends Schema.TaggedErrorClass<Forbidden>(
	`${ReasonTypeId}/Forbidden`,
)("Forbidden", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActorNotFound extends Schema.TaggedErrorClass<ActorNotFound>(
	`${ReasonTypeId}/ActorNotFound`,
)("ActorNotFound", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActorStopping extends Schema.TaggedErrorClass<ActorStopping>(
	`${ReasonTypeId}/ActorStopping`,
)("ActorStopping", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActorRestarting extends Schema.TaggedErrorClass<ActorRestarting>(
	`${ReasonTypeId}/ActorRestarting`,
)("ActorRestarting", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActionNotFound extends Schema.TaggedErrorClass<ActionNotFound>(
	`${ReasonTypeId}/ActionNotFound`,
)("ActionNotFound", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActionTimedOut extends Schema.TaggedErrorClass<ActionTimedOut>(
	`${ReasonTypeId}/ActionTimedOut`,
)("ActionTimedOut", { cause: Schema.instanceOf(RivetkitErrors.RivetError) }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActionAborted extends Schema.TaggedErrorClass<ActionAborted>(
	`${ReasonTypeId}/ActionAborted`,
)("ActionAborted", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class ActorOverloaded extends Schema.TaggedErrorClass<ActorOverloaded>(
	`${ReasonTypeId}/ActorOverloaded`,
)("ActorOverloaded", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class IncomingMessageTooLong extends Schema.TaggedErrorClass<IncomingMessageTooLong>(
	`${ReasonTypeId}/IncomingMessageTooLong`,
)("IncomingMessageTooLong", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class OutgoingMessageTooLong extends Schema.TaggedErrorClass<OutgoingMessageTooLong>(
	`${ReasonTypeId}/OutgoingMessageTooLong`,
)("OutgoingMessageTooLong", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class InvalidEncoding extends Schema.TaggedErrorClass<InvalidEncoding>(
	`${ReasonTypeId}/InvalidEncoding`,
)("InvalidEncoding", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class InvalidRequest extends Schema.TaggedErrorClass<InvalidRequest>(
	`${ReasonTypeId}/InvalidRequest`,
)("InvalidRequest", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class GuardError extends Schema.TaggedErrorClass<GuardError>(
	`${ReasonTypeId}/GuardError`,
)("GuardError", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
	}
}

export class InternalError extends Schema.TaggedErrorClass<InternalError>(
	`${ReasonTypeId}/InternalError`,
)("InternalError", {
	cause: Schema.instanceOf(RivetkitErrors.RivetError),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
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
)("UnknownUserError", { cause: Schema.instanceOf(RivetkitErrors.RivetError) }) {
	readonly [ReasonTypeId] = ReasonTypeId;
	override get message() {
		return this.cause.message;
	}
	get group() {
		return this.cause.group;
	}
	get code() {
		return this.cause.code;
	}
	get metadata() {
		return this.cause.metadata;
	}
	get actor() {
		return this.cause.actor;
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
	cause: Schema.Union([
		Schema.instanceOf(RivetkitErrors.RivetError),
		Schema.Defect,
	]),
}) {
	readonly [ReasonTypeId] = ReasonTypeId;
	get group() {
		return this.cause instanceof RivetkitErrors.RivetError
			? this.cause.group
			: undefined;
	}
	get code() {
		return this.cause instanceof RivetkitErrors.RivetError
			? this.cause.code
			: undefined;
	}
	get metadata() {
		return this.cause instanceof RivetkitErrors.RivetError
			? this.cause.metadata
			: undefined;
	}
	get actor() {
		return this.cause instanceof RivetkitErrors.RivetError
			? this.cause.actor
			: undefined;
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
	| IncomingMessageTooLong
	| OutgoingMessageTooLong
	| InvalidEncoding
	| InvalidRequest
	| GuardError
	| InternalError
	| UnknownUserError
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
		typeof IncomingMessageTooLong,
		typeof OutgoingMessageTooLong,
		typeof InvalidEncoding,
		typeof InvalidRequest,
		typeof GuardError,
		typeof InternalError,
		typeof UnknownUserError,
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
	IncomingMessageTooLong,
	OutgoingMessageTooLong,
	InvalidEncoding,
	InvalidRequest,
	GuardError,
	InternalError,
	UnknownUserError,
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

	override get message() {
		return this.reason.message || this.reason._tag;
	}
}

export const isRivetError = (u: unknown): u is RivetError =>
	Predicate.hasProperty(u, TypeId);

const reasonFromRivetkitRivetError = (
	error: RivetkitErrors.RivetError,
): Reason => {
	switch (`${error.group}.${error.code}`) {
		case `auth.${RivetkitErrors.forbiddenError().code}`:
			return new Forbidden({ cause: error });
		case `actor.${RivetkitErrors.actorNotFound().code}`:
			return new ActorNotFound({ cause: error });
		case `actor.${RivetkitErrors.actorStopping().code}`:
			return new ActorStopping({ cause: error });
		case `actor.${RivetkitErrors.actorRestarting().code}`:
			return new ActorRestarting({ cause: error });
		case `actor.action_not_found`:
			return new ActionNotFound({ cause: error });
		case `actor.action_timed_out`:
			return new ActionTimedOut({ cause: error });
		case `actor.aborted`:
			return new ActionAborted({ cause: error });
		case `actor.overloaded`:
			return new ActorOverloaded({ cause: error });
		case `actor.${RivetkitErrors.INTERNAL_ERROR_CODE}`:
		case `core.${RivetkitErrors.INTERNAL_ERROR_CODE}`:
		case `rivetkit.${RivetkitErrors.INTERNAL_ERROR_CODE}`:
			return new InternalError({ cause: error });
		case `message.incoming_too_long`:
			return new IncomingMessageTooLong({ cause: error });
		case `message.outgoing_too_long`:
			return new OutgoingMessageTooLong({ cause: error });
		case `encoding.${RivetkitErrors.invalidEncoding().code}`:
			return new InvalidEncoding({ cause: error });
		case `request.${RivetkitErrors.invalidRequest().code}`:
			return new InvalidRequest({ cause: error });
	}

	// Group-wide fallbacks: any code under the group maps to a single reason.
	if (error.group === "guard") return new GuardError({ cause: error });
	if (error.group === "user") return new UnknownUserError({ cause: error });

	return new UnknownError({
		message: error.message,
		cause: error,
	});
};

export const fromRivetkitRivetError = (
	e: RivetkitErrors.RivetError,
): RivetError => new RivetError({ reason: reasonFromRivetkitRivetError(e) });

export const fromUnknown = (cause: unknown): RivetError => {
	if (isRivetError(cause)) return cause;
	return fromRivetkitRivetError(RivetkitErrors.toRivetError(cause));
};
