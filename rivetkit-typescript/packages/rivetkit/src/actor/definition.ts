import type { RegistryConfig } from "@/registry/config";
import { ActorConfigSchema, type Actions, type ActorConfig, type ActorConfigInput } from "./config";
import { loggerWithoutContext } from "./log";
import type { AnyDatabaseProvider } from "@/common/database/config";
import type { EventSchemaConfig, QueueSchemaConfig } from "./schema";

const warnedDeprecatedTimeoutKeys = new Set<string>();

function warnDeprecatedShutdownTimeoutKeys(options: unknown) {
	if (!options || typeof options !== "object") return;
	const opts = options as Record<string, unknown>;
	for (const key of ["onDestroyTimeout", "waitUntilTimeout"]) {
		if (opts[key] !== undefined && !warnedDeprecatedTimeoutKeys.has(key)) {
			warnedDeprecatedTimeoutKeys.add(key);
			loggerWithoutContext().warn({
				msg: `actor option \`${key}\` is deprecated and is now ignored. Configure \`sleepGracePeriod\` instead, which bounds the entire graceful shutdown window for both sleep and destroy. Will be removed in 2.2.0.`,
			});
		}
	}
}

export interface BaseActorDefinition<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
	R extends Actions<S, CP, CS, V, I, DB, E, Q> = Actions<
		S,
		CP,
		CS,
		V,
		I,
		DB,
		E,
		Q
	>,
> {
	readonly config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;
}

export type AnyActorDefinition = BaseActorDefinition<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export type AnyStaticActorDefinition = ActorDefinition<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export class ActorDefinition<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
	R extends Actions<S, CP, CS, V, I, DB, E, Q> = Actions<
		S,
		CP,
		CS,
		V,
		I,
		DB,
		E,
		Q
	>,
> implements BaseActorDefinition<S, CP, CS, V, I, DB, E, Q, R>
{
	#config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;

	constructor(config: ActorConfig<S, CP, CS, V, I, DB, E, Q>) {
		this.#config = config;
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB, E, Q> {
		return this.#config;
	}
}

export interface BaseActorInstance<
	S = any,
	CP = any,
	CS = any,
	V = any,
	I = any,
	DB extends AnyDatabaseProvider = AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
> {
	id: string;
	config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;
	rLog: Record<string, (...args: any[]) => any>;
	[key: string]: any;
}

export type AnyActorInstance = BaseActorInstance<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export type AnyStaticActorInstance = AnyActorInstance;

export function isStaticActorInstance(
	_actor: AnyActorInstance,
): _actor is AnyStaticActorInstance {
	return true;
}

export function actor<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> = Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues,
		TActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues,
	TActions
> {
	warnDeprecatedShutdownTimeoutKeys(
		(input as { options?: unknown } | undefined)?.options,
	);
	const config = ActorConfigSchema.parse(input) as ActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	return new ActorDefinition(config);
}

export function isStaticActorDefinition(
	definition: AnyActorDefinition,
): definition is AnyStaticActorDefinition {
	return definition instanceof ActorDefinition;
}

export function lookupInRegistry(
	config: RegistryConfig,
	name: string,
): AnyActorDefinition {
	const definition = config.use[name];
	if (!definition) throw new Error(`no actor in registry for name ${name}`);
	return definition;
}
