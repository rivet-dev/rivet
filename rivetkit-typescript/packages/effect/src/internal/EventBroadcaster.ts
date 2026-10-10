import { Effect, Record, Schema } from "effect";
import type * as Event from "../Event.ts";

/**
 * Schema-typed view over an actor's declared events. Encodes a
 * broadcast payload through the matching event's schema before
 * handing it to RivetKit's raw `broadcast`.
 */
export interface EventBroadcaster<Events extends Event.Any> {
	readonly broadcast: <Tag extends Event.Tag<Events>>(
		tag: Tag,
		payload: Event.PayloadConstructor<Event.ExtractTag<Events, Tag>>,
	) => Effect.Effect<
		void,
		never,
		Event.ServicesServer<Event.ExtractTag<Events, Tag>>
	>;
}

/**
 * Builds an `EventBroadcaster` over `events`, calling `rawBroadcast`
 * (RivetKit's `ActorContext.broadcast`) with the encoded payload.
 *
 * Encode failures die rather than surface as a typed error, matching
 * `ActionDispatcher`'s treatment of outbound (success/error) encoding:
 * a payload that doesn't satisfy its own declared schema is a
 * programmer error, not an expected runtime failure.
 */
export const make = <Events extends Event.AnyWithProps>(
	events: ReadonlyArray<Events>,
	rawBroadcast: (name: string, ...args: ReadonlyArray<unknown>) => void,
): EventBroadcaster<Events> => {
	const encoders = Record.fromIterableWith(events, (event) => [
		event._tag,
		Schema.encodeEffect(Schema.toCodecJson(event.payloadSchema)),
	]);

	return {
		broadcast: (tag: string, payload: unknown) =>
			Effect.gen(function* () {
				const encode = encoders[tag];
				const encodedPayload = yield* encode(payload);
				rawBroadcast(tag, encodedPayload);
			}).pipe(Effect.orDie),
	} as EventBroadcaster<Events>;
};
