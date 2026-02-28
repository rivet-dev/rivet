import type { ActorKey } from "@/actor/mod";
import type { AnyActorDefinition } from "@/actor/definition";
import type { AnyClient, Client } from "@/client/client";
import type { Registry } from "@/registry";

export interface DynamicNodeProcessConfig {
	memoryLimit?: number;
	cpuTimeLimitMs?: number;
}

export interface DynamicActorLoadResult {
	source: string;
	nodeProcess?: DynamicNodeProcessConfig;
}

export interface DynamicActorLoaderContext {
	actorId: string;
	name: string;
	key: ActorKey;
	input: unknown;
	region: string;
	client<R extends Registry<any>>(): Client<R>;
}

export type DynamicActorLoader = (
	context: DynamicActorLoaderContext,
) => Promise<DynamicActorLoadResult> | DynamicActorLoadResult;

export interface DynamicActorMetadata {
	loader: DynamicActorLoader;
}

export const DYNAMIC_ACTOR_METADATA_SYMBOL = Symbol.for(
	"rivetkit.dynamic_actor.metadata",
);

export function attachDynamicActorMetadata(
	definition: AnyActorDefinition,
	metadata: DynamicActorMetadata,
): void {
	(
		definition as AnyActorDefinition & {
			[DYNAMIC_ACTOR_METADATA_SYMBOL]?: DynamicActorMetadata;
		}
	)[DYNAMIC_ACTOR_METADATA_SYMBOL] = metadata;
}

export function getDynamicActorMetadata(
	definition: AnyActorDefinition,
): DynamicActorMetadata | undefined {
	return (
		definition as AnyActorDefinition & {
			[DYNAMIC_ACTOR_METADATA_SYMBOL]?: DynamicActorMetadata;
		}
	)[DYNAMIC_ACTOR_METADATA_SYMBOL];
}

export function isDynamicActorDefinition(
	definition: AnyActorDefinition,
): boolean {
	return getDynamicActorMetadata(definition) !== undefined;
}

export function createDynamicActorLoaderContext(
	inlineClient: AnyClient,
	actorId: string,
	name: string,
	key: ActorKey,
	input: unknown,
	region: string,
): DynamicActorLoaderContext {
	return {
		actorId,
		name,
		key,
		input,
		region,
		client<R extends Registry<any>>(): Client<R> {
			return inlineClient as Client<R>;
		},
	};
}
