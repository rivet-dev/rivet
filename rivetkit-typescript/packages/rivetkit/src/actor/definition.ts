import type { RegistryConfig } from "@/registry/config";
import type { Actions, ActorConfig } from "./config";
import type { ActionContextOf, ActorContext } from "./contexts";
import type { AnyDatabaseProvider } from "./database";
import { ActorInstance } from "./instance/mod";
import { DeepMutable } from "@/utils";

export type AnyActorDefinition = ActorDefinition<
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
	R extends Actions<S, CP, CS, V, I, DB>,
> {
	#config: ActorConfig<S, CP, CS, V, I, DB>;

	constructor(config: ActorConfig<S, CP, CS, V, I, DB>) {
		this.#config = config;
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB> {
		return this.#config;
	}

	instantiate(): ActorInstance<S, CP, CS, V, I, DB> {
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
