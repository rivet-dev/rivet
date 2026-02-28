import {
	DynamicActorDefinition,
	type DynamicActorConfigInput,
	type DynamicActorLoader,
	type DynamicActorLoaderContext,
	type DynamicActorLoadResult,
	type DynamicNodeProcessConfig,
} from "./internal";
import type { DynamicSourceFormat } from "./runtime-bridge";

export function dynamicActor<TInput = unknown>(
	loader: DynamicActorLoader<TInput>,
	config: DynamicActorConfigInput = {},
): DynamicActorDefinition<TInput> {
	return new DynamicActorDefinition(loader, config);
}

export type {
	DynamicActorConfigInput,
	DynamicActorLoader,
	DynamicActorLoaderContext,
	DynamicActorLoadResult,
	DynamicNodeProcessConfig,
	DynamicSourceFormat,
};
