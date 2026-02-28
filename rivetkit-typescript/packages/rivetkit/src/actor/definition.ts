import type { RegistryConfig } from "@/registry/config";
import type { Actions, ActorConfig } from "./config";
import type { AnyDatabaseProvider } from "./database";
import {
	StaticActorInstance,
	type ActorInstance,
} from "./instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "./schema";

export interface ActorDefinition<
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
	instantiate():
		| ActorInstance<S, CP, CS, V, I, DB, E, Q>
		| Promise<ActorInstance<S, CP, CS, V, I, DB, E, Q>>;
}

export type AnyActorDefinition = ActorDefinition<
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

export class StaticActorDefinition<
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
> implements ActorDefinition<S, CP, CS, V, I, DB, E, Q, R> {
	#config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;

	constructor(config: ActorConfig<S, CP, CS, V, I, DB, E, Q>) {
		this.#config = config;
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB, E, Q> {
		return this.#config;
	}

	instantiate(): StaticActorInstance<S, CP, CS, V, I, DB, E, Q> {
		return new StaticActorInstance(this.#config);
	}
}

export function lookupInRegistry(
	config: RegistryConfig,
	name: string,
): AnyActorDefinition {
	const definition = config.use[name];
	if (!definition) throw new Error(`no actor in registry for name ${name}`);
	return definition;
}
