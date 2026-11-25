import { Schema } from "effect";

const TypeId = "~@rivetkit/effect/Event";

/**
 * A value-level definition for a message an actor pushes to its connections.
 * The tag is the RivetKit event name, so raw RivetKit clients receive it with
 * `conn.on(tag, ...)`.
 */
export interface Event<Tag extends string, Payload extends Schema.Top> {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: Tag;
	/**
	 * A no-payload event goes out without arguments, like a raw RivetKit
	 * event, instead of as the JSON Void codec's null.
	 */
	readonly hasPayload: boolean;
	readonly payloadSchema: Payload;
}

/** Type-erased view of any `Event`. */
export interface Any {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly hasPayload: boolean;
	readonly payloadSchema: Schema.Top;
}

export type PayloadSchema<E> =
	E extends Event<infer _Tag, infer _Payload> ? _Payload : never;

/** The shape accepted by the payload schema's `make` constructor, before encoding. */
export type PayloadConstructor<E> = PayloadSchema<E>["~type.make.in"];

const Proto = {
	[TypeId]: TypeId,
};

/**
 * Define a Rivet Actor event.
 *
 * @example
 * ```ts
 * import { Schema } from "effect"
 * import { Event } from "@rivetkit/effect"
 *
 * export const CountChanged = Event.make("CountChanged", {
 *   payload: { count: Schema.Number },
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
	return Object.assign(Object.create(Proto), {
		_tag: tag,
		hasPayload: options?.payload !== undefined,
		payloadSchema,
	});
};
