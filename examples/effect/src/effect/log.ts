import { Effect } from "effect";
import { ActorContextTag, context } from "./actor.ts";

// Log namespace for structured logging
export namespace Log {
	export const info = (message: string): Effect.Effect<void, never, typeof ActorContextTag> =>
		Effect.gen(function* () {
			const c = yield* context();
			c.log.info(message);
		});

	export const warn = (message: string): Effect.Effect<void, never, typeof ActorContextTag> =>
		Effect.gen(function* () {
			const c = yield* context();
			c.log.warn(message);
		});

	export const error = (message: string): Effect.Effect<void, never, typeof ActorContextTag> =>
		Effect.gen(function* () {
			const c = yield* context();
			c.log.error(message);
		});

	export const debug = (message: string): Effect.Effect<void, never, typeof ActorContextTag> =>
		Effect.gen(function* () {
			const c = yield* context();
			c.log.debug(message);
		});
}
