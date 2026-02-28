import {
	DynamicActorDefinition,
	type DynamicActorConfigInput,
	type DynamicActorAuth,
	type DynamicActorAuthContext,
	type DynamicActorLoader,
	type DynamicActorLoaderContext,
	type DynamicActorLoadResult,
	type DynamicNodeProcessConfig,
	type DynamicActorOptionsInput,
} from "./internal";
import type { DynamicSourceFormat } from "./runtime-bridge";

export function dynamicActor<TInput = unknown, TConnParams = unknown>(
	config: DynamicActorConfigInput<TInput, TConnParams>,
): DynamicActorDefinition<TInput, TConnParams> {
	return new DynamicActorDefinition(config);
}

export type {
	DynamicActorAuth,
	DynamicActorAuthContext,
	DynamicActorConfigInput,
	DynamicActorLoader,
	DynamicActorLoaderContext,
	DynamicActorLoadResult,
	DynamicNodeProcessConfig,
	DynamicActorOptionsInput,
	DynamicSourceFormat,
};
