import * as Predicate from "effect/Predicate";
import * as Schema from "effect/Schema";

const TypeId = "~@rivetkit/effect/Message";

const EnvelopeTypeId = "~@rivetkit/effect/Message/Envelope";

export const isMessage = (u: unknown): u is Message<any, any, any> =>
	Predicate.hasProperty(u, TypeId);

/**
 * A Rivet Actor message: a durable, queued operation that the actor
 * processes asynchronously on its main loop.
 *
 * @remarks
 *
 * `Message` is a value-level definition that carries the wire schemas
 * for the request payload and, optionally, a completion response. The
 * message's implementation lives in the actor's handler map; this type
 * only describes the contract.
 *
 * Messages come in two flavors, distinguished at the type level by the
 * `Success` schema:
 *
 * - **Non-completable** (fire-and-forget). `Success` defaults to
 *    `Schema.Never`. Sending returns once the message is durably
 *    enqueued; the sender does not observe the actor's processing.
 *
 * - **Completable**. `Success` is provided. Sending returns an Effect
 *    that resolves with the typed completion value once the actor's
 *    handler invokes its `complete` callback.
 *
 * Unlike `Action`, `Message` has no error channel. A message may sit
 * in the queue long after the sender has moved on, so propagating a
 * typed error back to the sender is not a meaningful contract. Handler
 * failures are surfaced through the actor's standard supervision and
 * retry mechanisms instead.
 *
 * `Message` values are callable: invoking the message with a payload
 * produces a typed `Envelope` that can be passed to `actor.send(...)`.
 *
 * @example
 * ```ts
 * import { Schema } from "effect"
 * import { Message } from "@rivetkit/effect"
 *
 * // Non-completable
 * export const Reset = Message.make("Reset", {
 *   payload: { reason: Schema.String },
 * })
 *
 * // Completable
 * export const IncrementBy = Message.make("IncrementBy", {
 *   payload: { amount: Schema.Number },
 *   success: Schema.Number,
 * })
 * ```
 */
export interface Message<
	in out Tag extends string,
	in out Payload extends Schema.Top = Schema.Void,
	out Success extends Schema.Top = Schema.Never,
> {
	(payload: Payload["~type.make.in"]): Envelope<Tag, Payload, Success>;

	readonly [TypeId]: typeof TypeId;
	readonly _tag: Tag;
	readonly key: string;
	readonly payloadSchema: Payload;
	readonly successSchema: Success;
	/**
	 * Whether this message yields a typed completion to the sender.
	 * `true` when a `success` schema was supplied; `false` for
	 * fire-and-forget messages.
	 */
	readonly completable: IsCompletable<Message<Tag, Payload, Success>>;
}

/**
 * A typed payload envelope produced by calling a `Message` value.
 *
 * The runtime uses `_tag` to dispatch to the correct handler. The
 * `Success` type parameter is phantom: it surfaces the completion
 * type at the call site so `actor.send(...)` can return a
 * precisely-typed Effect without a second lookup.
 */
export interface Envelope<
	in out Tag extends string,
	out Payload extends Schema.Top = Schema.Void,
	out Success extends Schema.Top = Schema.Never,
> {
	readonly [EnvelopeTypeId]: typeof EnvelopeTypeId;
	readonly _tag: Tag;
	readonly payload: Payload["Type"];
	readonly "~successSchema": Success;
}

/**
 * Type-erased view of any `Message`. Useful for collections of
 * messages where the specific schemas don't matter.
 */
export interface Any {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
	readonly completable: boolean;
}

/**
 * Like `Any`, but with the prop fields (`*Schema`) accessible. Used
 * by internal builders that need to read schemas off a message.
 */
export interface AnyWithProps {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
	readonly payloadSchema: Schema.Top;
	readonly successSchema: Schema.Top;
	readonly completable: boolean;
}

/**
 * Type-erased view of any `Envelope`.
 */
export interface AnyEnvelope {
	readonly [EnvelopeTypeId]: typeof EnvelopeTypeId;
	readonly _tag: string;
	readonly payload: unknown;
}

// --- Type helpers ---------------------------------------------------

export type Tag<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Tag
		: never;

export type PayloadSchema<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Payload
		: never;

export type Payload<R> = PayloadSchema<R>["Type"];

/**
 * The shape accepted by the payload schema's `make` constructor on
 * the client side (i.e. before encoding). Useful for typing the
 * call site.
 */
export type PayloadConstructor<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Payload["~type.make.in"]
		: never;

export type SuccessSchema<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Success
		: never;

export type Success<R> = SuccessSchema<R>["Type"];

/**
 * `true` when the message is completable (a `success` schema was
 * provided), `false` for fire-and-forget messages.
 *
 * Driven off `Schema.Never` because `Schema.Void` is a legitimate
 * completion type meaning "the sender awaits completion but the
 * value carries no information."
 */
export type IsCompletable<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? [_Success] extends [typeof Schema.Never]
			? false
			: true
		: never;

/**
 * The full set of decoding/encoding services required by every
 * schema referenced by the message. Code generators include this in
 * the `R` channel of any effect that handles or sends the message.
 */
export type Services<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		?
				| _Payload["DecodingServices"]
				| _Payload["EncodingServices"]
				| _Success["DecodingServices"]
				| _Success["EncodingServices"]
		: never;

/**
 * The subset of `Services` actually needed on the client side:
 * encoding the payload, decoding the (optional) completion response.
 */
export type ServicesClient<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Payload["EncodingServices"] | _Success["DecodingServices"]
		: never;

/**
 * The subset of `Services` needed on the server side: decoding the
 * payload, encoding the (optional) completion response.
 */
export type ServicesServer<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? _Payload["DecodingServices"] | _Success["EncodingServices"]
		: never;

/**
 * Extract the message with the matching tag from a union of
 * messages.
 */
export type ExtractTag<R extends Any, Tag extends string> = R extends Message<
	Tag,
	infer _Payload,
	infer _Success
>
	? R
	: never;

/**
 * Extract the envelope union for a union of messages. Useful for
 * typing an actor's message-queue handler.
 */
export type EnvelopeOf<R> =
	R extends Message<infer _Tag, infer _Payload, infer _Success>
		? Envelope<_Tag, _Payload, _Success>
		: never;

// --- Implementation -------------------------------------------------

const Proto = {
	[TypeId]: TypeId,
};

const makeProto = <
	const Tag extends string,
	Payload extends Schema.Top,
	Success extends Schema.Top,
>(options: {
	readonly _tag: Tag;
	readonly payloadSchema: Payload;
	readonly successSchema: Success;
	readonly completable: boolean;
}): Message<Tag, Payload, Success> => {
	function Message(payload: unknown) {
		return {
			[EnvelopeTypeId]: EnvelopeTypeId,
			_tag: options._tag,
			payload: (options.payloadSchema as any).make(payload),
		};
	}
	Object.setPrototypeOf(Message, Proto);
	Object.assign(Message, options);
	Message.key = `@rivetkit/effect/Message/${options._tag}`;
	return Message as any;
};

/**
 * Define a Rivet Actor message.
 *
 * Omit `success` for a fire-and-forget message. Provide it (even as
 * `Schema.Void`) to make the message completable: the sender awaits
 * the actor's `complete` callback and receives the typed value.
 *
 * @example
 * ```ts
 * import { Schema } from "effect"
 * import { Message } from "@rivetkit/effect"
 *
 * // Fire-and-forget
 * export const Reset = Message.make("Reset", {
 *   payload: { reason: Schema.String },
 * })
 *
 * // Completable
 * export const IncrementBy = Message.make("IncrementBy", {
 *   payload: { amount: Schema.Number },
 *   success: Schema.Number,
 * })
 * ```
 */
export const make = <
	const Tag extends string,
	Payload extends Schema.Top | Schema.Struct.Fields = Schema.Void,
	Success extends Schema.Top = typeof Schema.Never,
>(
	tag: Tag,
	options?: {
		readonly payload?: Payload;
		readonly success?: Success;
	},
): Message<
	Tag,
	Payload extends Schema.Struct.Fields ? Schema.Struct<Payload> : Payload,
	Success
> => {
	const successSchema = options?.success ?? Schema.Never;
	const completable = options?.success !== undefined;
	const payloadSchema: Schema.Top = Schema.isSchema(options?.payload)
		? (options?.payload as any)
		: options?.payload
			? Schema.Struct(options?.payload as any)
			: Schema.Void;
	return makeProto({
		_tag: tag,
		payloadSchema,
		successSchema,
		completable,
	}) as any;
};
