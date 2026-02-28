import { actor } from "@/actor/mod";
import type { AnyActorDefinition } from "@/actor/definition";
import {
	attachDynamicActorMetadata,
	type DynamicActorLoader,
	type DynamicActorLoaderContext,
	type DynamicActorLoadResult,
	type DynamicNodeProcessConfig,
} from "./internal";

export function dynamicActor(loader: DynamicActorLoader): AnyActorDefinition {
	const definition = actor({
		// Keep the host-side placeholder actor awake. Sleep/wake semantics
		// are handled by the evaluated actor inside the isolate runtime.
		options: {
			noSleep: true,
		},
	}) as AnyActorDefinition;

	attachDynamicActorMetadata(definition, { loader });
	return definition;
}

export type {
	DynamicActorLoader,
	DynamicActorLoaderContext,
	DynamicActorLoadResult,
	DynamicNodeProcessConfig,
};
