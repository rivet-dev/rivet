import { Effect } from "effect";
import { RivetActorContext } from "./actor.ts";

type LogLevel = "info" | "warn" | "error" | "debug";

const logWithLevel = (
	level: LogLevel,
	message: string,
	props?: Record<string, unknown>,
) =>
	Effect.gen(function* () {
		const ctx = yield* RivetActorContext;
		ctx.log[level]({ msg: message, ...props });
	});

export const info = (message: string, props?: Record<string, unknown>) =>
	logWithLevel("info", message, props);

export const warn = (message: string, props?: Record<string, unknown>) =>
	logWithLevel("warn", message, props);

export const error = (message: string, props?: Record<string, unknown>) =>
	logWithLevel("error", message, props);

export const debug = (message: string, props?: Record<string, unknown>) =>
	logWithLevel("debug", message, props);
