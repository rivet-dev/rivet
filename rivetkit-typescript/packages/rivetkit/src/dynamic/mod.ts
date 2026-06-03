import { ActorConfigSchema } from "@/actor/config";
import type { AnyActorDefinition } from "@/actor/definition";
import {
	DYNAMIC_ACTOR_DEFINITION_SYMBOL,
	type DynamicActorDefinition,
	type DynamicActorLoader,
} from "./internal";

export interface DynamicActorConfig {
	load: DynamicActorLoader;
}

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
	};
}

export type {
	DynamicActorDefinition,
	DynamicActorLoadContext,
	DynamicActorLoader,
	DynamicActorSource,
} from "./internal";
