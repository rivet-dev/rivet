import { ActorConfigSchema } from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import {
	DYNAMIC_ACTOR_DEFINITION_SYMBOL,
	type DynamicActorDefinition,
	type DynamicActorLoader,
	type DynamicActorOptions,
} from "./internal";

export interface DynamicActorConfig {
	/**
	 * Resolves the actor source when an instance starts. Runs in the host
	 * process; the returned source executes in a dedicated worker thread.
	 */
	load: DynamicActorLoader;
	options?: DynamicActorOptions;
}

/**
 * Define an actor whose source code is resolved at actor start time.
 *
 * The loader runs per actor instance and returns source text that must
 * export an actor definition as its default export. The source executes in
 * its own `node:worker_threads` worker, bridged to the host runtime; state,
 * KV, SQLite, and lifecycle stay in the host process.
 */
export function dynamicActor(
	input: DynamicActorConfig,
): DynamicActorDefinition {
	const config = ActorConfigSchema.parse({
		actions: {},
	}) as unknown as AnyActorDefinition["config"];

	return {
		[DYNAMIC_ACTOR_DEFINITION_SYMBOL]: true,
		config,
		loader: input.load,
		options: input.options ?? {},
	};
}

export type {
	DynamicActorDefinition,
	DynamicActorLoadContext,
	DynamicActorLoader,
	DynamicActorLoadResult,
	DynamicActorOptions,
} from "./internal";
