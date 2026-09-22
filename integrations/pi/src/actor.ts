import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import {
	type Actions,
	type ActorConfigInput,
	type ActorDefinition,
	actor,
	event,
	type EventSchemaConfig,
	type QueueSchemaConfig,
	type Type,
} from "rivetkit";
import { db } from "rivetkit/db";
import { createPiActions, type PiActions } from "./actions.js";
import {
	closePiSession,
	createPiRuntime,
	PI_RUNTIME,
	type PiContext,
	type PiDatabaseProvider,
	type PiSessionOptions,
} from "./runtime.js";
import { migratePiTables } from "./storage.js";

/** Ten minutes. Pi actions such as `waitForIdle` and `compact` outlive RivetKit's one-minute default. */
const DEFAULT_ACTION_TIMEOUT_MS = 10 * 60_000;

/** Every Pi `AgentSessionEvent`, in order, for connected clients. */
export type PiEvents = {
	event: Type<AgentSessionEvent>;
};

const piEvents: PiEvents = {
	event: event<AgentSessionEvent>(),
};

const piOptionKeys = [
	"cwd",
	"agentDir",
	"modelRuntime",
	"model",
	"thinkingLevel",
	"scopedModels",
	"noTools",
	"tools",
	"excludeTools",
	"customTools",
	"resourceLoader",
	"sessionStartEvent",
	"settings",
] as const satisfies readonly (keyof PiSessionOptions)[];

export type PiActorConfigInput<
	TState = undefined,
	TConnParams = undefined,
	TConnState = undefined,
	TVars = undefined,
	TInput = undefined,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TUserActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		PiDatabaseProvider,
		TEvents,
		TQueues
	> = Record<never, never>,
> = ActorConfigInput<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	PiDatabaseProvider,
	TEvents,
	TQueues,
	TUserActions
> &
	PiSessionOptions;

/**
 * The actions the user defined. With no `actions` option, TypeScript uses the
 * default, a record with an index signature, which would hide the Pi actions.
 */
type UserActions<T> = string extends keyof T ? Record<never, never> : T;

/**
 * Defines a Rivet Actor that owns one Pi coding-agent session.
 *
 * The session transcript and settings live in the actor's SQLite database and
 * are restored when the actor wakes. Pi events are broadcast on `event`.
 * Ordinary actor config (state, vars, actions, events, hooks) is passed through.
 */
export function pi<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TUserActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		PiDatabaseProvider,
		TEvents,
		TQueues
	> = Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		PiDatabaseProvider,
		TEvents,
		TQueues
	>,
>(
	config: PiActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TEvents,
		TQueues,
		TUserActions
	> = {} as PiActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TEvents,
		TQueues,
		TUserActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	PiDatabaseProvider,
	TEvents & PiEvents,
	TQueues,
	UserActions<TUserActions> & PiActions
> {
	const { actorConfig, sessionOptions } = splitConfig(config);
	if (actorConfig.db !== undefined) {
		throw new Error("pi() owns the actor database; remove the db option");
	}
	const actions = createPiActions(sessionOptions);
	assertNoReservedKeys("action", actorConfig.actions, actions);
	assertNoReservedKeys("event", actorConfig.events, piEvents);

	const userVars = actorConfig.vars;
	const userCreateVars = actorConfig.createVars;
	const userOnSleep = actorConfig.onSleep;
	const userOnDestroy = actorConfig.onDestroy;
	delete actorConfig.vars;

	return actor({
		...actorConfig,
		options: {
			actionTimeout: DEFAULT_ACTION_TIMEOUT_MS,
			...actorConfig.options,
		},
		db: db({ onMigrate: migratePiTables }),
		events: { ...(actorConfig.events ?? {}), ...piEvents },
		actions: { ...(actorConfig.actions ?? {}), ...actions },
		createVars: async (c: unknown, driverCtx: unknown) => {
			const vars = userCreateVars
				? await userCreateVars(c, driverCtx)
				: userVars === undefined
					? undefined
					: structuredClone(userVars);
			return attachRuntime(vars);
		},
		onSleep: async (c: PiContext) => {
			try {
				await userOnSleep?.(c);
			} finally {
				await closePiSession(c);
			}
		},
		onDestroy: async (c: PiContext) => {
			try {
				await userOnDestroy?.(c);
			} finally {
				await closePiSession(c);
			}
		},
	} as any) as ActorDefinition<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		PiDatabaseProvider,
		TEvents & PiEvents,
		TQueues,
		UserActions<TUserActions> & PiActions
	>;
}

function splitConfig(config: object): {
	actorConfig: Record<string, any>;
	sessionOptions: PiSessionOptions;
} {
	const actorConfig: Record<string, any> = { ...config };
	const sessionOptions: Record<string, unknown> = {};
	for (const key of piOptionKeys) {
		if (key in actorConfig) {
			sessionOptions[key] = actorConfig[key];
			delete actorConfig[key];
		}
	}
	return { actorConfig, sessionOptions: sessionOptions as PiSessionOptions };
}

/** Adds the Pi runtime slot to the user's vars without changing their shape. */
function attachRuntime(vars: unknown): object {
	const runtime = createPiRuntime();
	if (vars === undefined) {
		return { [PI_RUNTIME]: runtime };
	}
	if (typeof vars !== "object" || vars === null) {
		throw new Error("pi() requires actor vars to be an object");
	}
	return Object.assign(vars, { [PI_RUNTIME]: runtime });
}

function assertNoReservedKeys(
	kind: string,
	custom: object | undefined,
	builtIns: object,
): void {
	for (const key of Object.keys(custom ?? {})) {
		if (key in builtIns) {
			throw new Error(`pi() ${kind} name is reserved: ${key}`);
		}
	}
}

