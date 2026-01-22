import { Effect } from "effect";
import { ActorContextTag } from "./actor.ts";

export const info = (message: string, props?: Record<string, unknown>) =>
	Effect.gen(function* () {
		const ctx = yield* ActorContextTag;
		ctx.log.info({ msg: message, ...props });
	});

export const warn = (message: string, props?: Record<string, unknown>) =>
	Effect.gen(function* () {
		const ctx = yield* ActorContextTag;
		ctx.log.warn({ msg: message, ...props });
	});

export const error = (message: string, props?: Record<string, unknown>) =>
	Effect.gen(function* () {
		const ctx = yield* ActorContextTag;
		ctx.log.error({ msg: message, ...props });
	});

export const debug = (message: string, props?: Record<string, unknown>) =>
	Effect.gen(function* () {
		const ctx = yield* ActorContextTag;
		ctx.log.debug({ msg: message, ...props });
	});
