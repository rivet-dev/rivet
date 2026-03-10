import type { RegistryConfig } from "@/registry/config";
import type { Actions, ActorConfig } from "./config";
import type { AnyDatabaseProvider } from "./database";
import { ActorInstance } from "./instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "./schema";

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
	> implements BaseActorDefinition<S, CP, CS, V, I, DB, E, Q, R> {
	#config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;

	constructor(config: ActorConfig<S, CP, CS, V, I, DB, E, Q>) {
		this.#config = config;
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB, E, Q> {
		return this.#config;
	}

	instantiate(): ActorInstance<S, CP, CS, V, I, DB, E, Q> {
		return new ActorInstance(this.#config);
	}
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
