import type { RegistryConfig } from "@/registry/config";
import { DeepMutable } from "@/utils";
import type { Actions, ActorConfig } from "./config";
import type { ActionContextOf, ActorContext } from "./contexts";
import type { AnyDatabaseProvider } from "./database";
import { ActorInstance } from "./instance/mod";
import type { SchemaConfig } from "./schema";

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

export class ActorDefinition<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends SchemaConfig = Record<never, never>,
	Q extends SchemaConfig = Record<never, never>,
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

export function lookupInRegistry(
	config: RegistryConfig,
	name: string,
): AnyActorDefinition {
	// Build actor
	const definition = config.use[name];
	if (!definition) throw new Error(`no actor in registry for name ${name}`);
	return definition;
}
