import {
	DynamicActorDefinition,
	type DynamicActorConfigInput,
	type DynamicActorAuth,
	type DynamicActorAuthContext,
	type DynamicActorCanReload,
	type DynamicActorReloadContext,
	type DynamicActorLoader,
	type DynamicActorLoaderContext,
	type DynamicActorLoadResult,
	type DynamicActorOptions,
	type DynamicNodeProcessConfig,
	type DynamicActorOptionsInput,
	type DynamicStartupOptions,
} from "./internal";
import type { DynamicSourceFormat } from "./runtime-bridge";
export { compileActorSource } from "./compile";
export type {
	CompileActorSourceOptions,
	CompileActorSourceResult,
	TypeScriptDiagnostic,
} from "./compile";
export {
	createDynamicRuntimeStatus,
	transitionToStarting,
	transitionToRunning,
	transitionToFailedStart,
	transitionToInactive,
} from "./runtime-status";
export type {
	DynamicRuntimeState,
	DynamicRuntimeStatus,
} from "./runtime-status";
export {
	coalesceDynamicStartup,
	computeBackoffDelay,
} from "./startup-coalescing";

export function dynamicActor<TInput = unknown, TConnParams = unknown>(
	config: DynamicActorConfigInput<TInput, TConnParams>,
): DynamicActorDefinition<TInput, TConnParams> {
	return new DynamicActorDefinition(config);
}

export type {
	DynamicActorAuth,
	DynamicActorAuthContext,
	DynamicActorCanReload,
	DynamicActorConfigInput,
	DynamicActorLoader,
	DynamicActorLoaderContext,
	DynamicActorLoadResult,
	DynamicActorOptions,
	DynamicActorReloadContext,
	DynamicNodeProcessConfig,
	DynamicActorOptionsInput,
	DynamicSourceFormat,
	DynamicStartupOptions,
};
