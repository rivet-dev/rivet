import type { AnyActorDefinition } from "@/actor/definition";
import {
	DynamicActorDefinition,
	type DynamicActorConfigInput,
	type DynamicActorLoader,
	type DynamicActorLoaderContext,
	type DynamicActorLoadResult,
	type DynamicNodeProcessConfig,
} from "./internal";

export function dynamicActor(
	loader: DynamicActorLoader,
	config: DynamicActorConfigInput = {},
): AnyActorDefinition {
	return new DynamicActorDefinition(loader, config);
}

export type {
	DynamicActorConfigInput,
	DynamicActorLoader,
	DynamicActorLoaderContext,
	DynamicActorLoadResult,
	DynamicNodeProcessConfig,
};
