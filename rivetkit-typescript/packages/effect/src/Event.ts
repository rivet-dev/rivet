import { Predicate, Schema } from "effect";

const TypeId = "~@rivetkit/effect/Event";

export const isEvent = (u: unknown): u is Event<any, any> =>
	Predicate.hasProperty(u, TypeId);

/**
 * A value-level definition for a broadcastable event. Declaring an
 * event registers its name with RivetKit and gives `Actor.toLayer`'s
 * wake options a schema-typed `events.broadcast` for it.
 */
export interface Event<
	Tag extends string,
	Payload extends Schema.Top = Schema.Void,
> {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: Tag;
	readonly key: string;
	readonly payloadSchema: Payload;
}

/**
 * Type-erased view of any `Event`. Useful for collections of events
 * where the specific schema doesn't matter.
 */
export interface Any {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
}

/**
 * Like `Any`, but with the prop fields (`payloadSchema`) accessible.
 * Used by internal builders that need to read the schema off an event.
 */
export interface AnyWithProps {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
	readonly payloadSchema: Schema.Top;
}

// --- Type helpers ---------------------------------------------------

export type Tag<R> = R extends Event<infer _Tag, infer _Payload> ? _Tag : never;

export type PayloadSchema<R> =
	R extends Event<infer _Tag, infer _Payload> ? _Payload : never;

export type Payload<R> = PayloadSchema<R>["Type"];

/**
 * The shape accepted by the payload schema's `make` constructor on the
 * broadcasting side (i.e. before encoding). Useful for typing the call
 * site of `events.broadcast(tag, payload)`.
 */
export type PayloadConstructor<R> =
	R extends Event<infer _Tag, infer _Payload>
		? _Payload["~type.make.in"]
		: never;

/**
 * The services required to encode this event's payload when
 * broadcasting from inside an actor.
 */
export type ServicesServer<R> =
	R extends Event<infer _Tag, infer _Payload>
		? _Payload["EncodingServices"]
		: never;

/**
 * The services required to decode this event's payload when
 * receiving it on a client.
 */
export type ServicesClient<R> =
	R extends Event<infer _Tag, infer _Payload>
		? _Payload["DecodingServices"]
		: never;

/**
 * Extract the event with the matching tag from a union of events.
 */
export type ExtractTag<R extends Any, Tag extends string> = R extends {
	readonly _tag: Tag;
}
	? R
	: never;

// --- Implementation -------------------------------------------------

const Proto = {
	[TypeId]: TypeId,
};

const makeProto = <
	const Tag extends string,
	Payload extends Schema.Top,
>(options: {
	readonly _tag: Tag;
	readonly payloadSchema: Payload;
}): Event<Tag, Payload> => {
	const self = Object.assign(Object.create(Proto), options);
	self.key = `@rivetkit/effect/Event/${options._tag}`;
	return self;
};

/**
 * Define a Rivet Actor event.
 *
 * @example
 * ```ts
 * import { Schema } from "effect"
 * import { Event } from "@rivetkit/effect"
 *
 * export const InboxChanged = Event.make("InboxChanged", {
 *   payload: { unread: Schema.Number },
 * })
 * ```
 */
export const make = <
	const Tag extends string,
	Payload extends Schema.Top | Schema.Struct.Fields = Schema.Void,
>(
	tag: Tag,
	options?: {
		readonly payload?: Payload;
	},
): Event<
	Tag,
	Payload extends Schema.Struct.Fields ? Schema.Struct<Payload> : Payload
> => {
	const payloadSchema: Schema.Top = Schema.isSchema(options?.payload)
		? (options?.payload as any)
		: options?.payload
			? Schema.Struct(options?.payload as any)
			: Schema.Void;
	return makeProto({
		_tag: tag,
		payloadSchema,
	}) as Event<
		Tag,
		Payload extends Schema.Struct.Fields ? Schema.Struct<Payload> : Payload
	>;
};
