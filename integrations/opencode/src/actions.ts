import type { OpenCode } from "@opencode/sdk/effect";
import { Effect, type Brand } from "effect";
import type { OpenCodeHost } from "./host.js";
import type { OpenCodeContext } from "./actor.js";

type Selection<T> = {
	[K in keyof T]?: T[K] extends (...args: any[]) => Effect.Effect<any, any>
		? true | Selection<T[K]>
		: T[K] extends object
			? Selection<T[K]>
			: never;
};
/** SDK brands are an in-process detail; actor inputs contain plain JSON. */
export type OpenCodeInput<T> =
	T extends Brand.Brand<any>
		? T extends string
			? string
			: T extends number
				? number
				: T
		: T extends object
			? { [K in keyof T]: OpenCodeInput<T[K]> }
			: T;

/** Explicit allowlist: no raw RPC, host administration, or unbounded streams. */
export const actionSurface = {
	session: {
		list: true,
		stats: true,
		create: true,
		import: true,
		export: true,
		active: true,
		get: true,
		remove: true,
		fork: true,
		switchAgent: true,
		switchModel: true,
		rename: true,
		move: true,
		prompt: true,
		command: true,
		skill: true,
		synthetic: true,
		shell: true,
		compact: true,
		wait: true,
		revert: { stage: true, clear: true, commit: true },
		context: true,
		inbox: { list: true, cancel: true, steer: true, queue: true },
		instructions: { entry: { list: true, put: true, remove: true } },
		generate: true,
		interrupt: true,
		background: true,
		message: true,
		environment: true,
		view: true,
	},
	message: { list: true },
	agent: { list: true, get: true },
	model: { list: true, default: true },
	provider: { list: true, get: true },
	command: { list: true },
	skill: { list: true },
	permission: {
		request: { list: true },
		saved: { list: true, remove: true },
		create: true,
		list: true,
		get: true,
		reply: true,
		rules: true,
	},
	form: {
		request: { list: true },
		list: true,
		create: true,
		get: true,
		state: true,
		reply: true,
		cancel: true,
	},
	mcp: {
		list: true,
		add: true,
		remove: true,
		connect: true,
		disconnect: true,
		resource: { catalog: true },
	},
	plugin: { list: true, awaitActivation: true, check: true, update: true },
	config: { get: true },
} as const satisfies Selection<OpenCode.Interface>;

type Forward<T, S> = {
	[K in keyof S & keyof T]: S[K] extends true
		? T[K] extends (...args: infer A) => Effect.Effect<infer R, any>
			? (context: OpenCodeContext, ...args: OpenCodeInput<A>) => Promise<R>
			: never
		: Forward<T[K], S[K]>;
};
export type OpenCodeNativeActions = Forward<
	OpenCode.Interface,
	typeof actionSurface
>;

export function nativeActions(
	invoke: (
		context: OpenCodeContext,
		path: string[],
		args: unknown[],
	) => Promise<unknown>,
): OpenCodeNativeActions {
	const build = (selection: object, parent: string[]): object =>
		Object.fromEntries(
			Object.entries(selection).map(([name, value]) => {
				const path = [...parent, name];
				return [
					name,
					value === true
						? (context: OpenCodeContext, ...args: unknown[]) =>
								invoke(context, path, args)
						: build(value, path),
				];
			}),
		);
	return build(actionSurface, []) as OpenCodeNativeActions;
}

export function invokeNative(
	host: OpenCodeHost,
	path: string[],
	args: unknown[],
): Promise<unknown> {
	let owner: any = host.client;
	for (const key of path.slice(0, -1)) owner = owner[key];
	const operation = owner[path.at(-1)!](...args);
	if (!Effect.isEffect(operation))
		throw new Error(
			`OpenCode action ${path.join(".")} is not an RPC operation`,
		);
	return Effect.runPromise(operation as Effect.Effect<unknown, unknown>);
}
