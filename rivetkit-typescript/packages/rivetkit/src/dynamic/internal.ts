import type { ActorKey } from "@/actor/mod";
import type {
	ActorConfig,
	GlobalActorOptionsInput,
} from "@/actor/config";
import { ActorConfigSchema } from "@/actor/config";
import type {
	AnyActorDefinition,
	BaseActorDefinition,
} from "@/actor/definition";
import type { AnyDatabaseProvider } from "@/actor/database";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";
import type { AnyClient, Client } from "@/client/client";
import type { Registry } from "@/registry";
import type { DynamicSourceFormat } from "./runtime-bridge";

export interface DynamicNodeProcessConfig {
	memoryLimit?: number;
	cpuTimeLimitMs?: number;
}

export interface DynamicActorLoadResult {
	/** Actor module source text returned by the dynamic loader. */
	source: string;
	/**
	 * Source format for `source`.
	 *
	 * Defaults to `typescript` for backward compatibility.
	 */
	sourceFormat?: DynamicSourceFormat;
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

export interface DynamicActorConfigInput {
	options?: GlobalActorOptionsInput;
}

export class DynamicActorDefinition
	implements
		BaseActorDefinition<
			any,
			any,
			any,
			any,
			any,
			AnyDatabaseProvider,
			EventSchemaConfig,
			QueueSchemaConfig,
			Record<string, (...args: any[]) => unknown>
		>
{
	#loader: DynamicActorLoader;
	#config: ActorConfig<
		any,
		any,
		any,
		any,
		any,
		AnyDatabaseProvider,
		EventSchemaConfig,
		QueueSchemaConfig
	>;

	constructor(loader: DynamicActorLoader, input: DynamicActorConfigInput = {}) {
		this.#loader = loader;
		this.#config = ActorConfigSchema.parse({
			actions: {},
			options: input.options ?? {},
		}) as ActorConfig<
			any,
			any,
			any,
			any,
			any,
			AnyDatabaseProvider,
			EventSchemaConfig,
			QueueSchemaConfig
		>;
	}

	get loader(): DynamicActorLoader {
		return this.#loader;
	}

	get config(): ActorConfig<
		any,
		any,
		any,
		any,
		any,
		AnyDatabaseProvider,
		EventSchemaConfig,
		QueueSchemaConfig
	> {
		return this.#config;
	}
}

export function isDynamicActorDefinition(
	definition: AnyActorDefinition,
): definition is DynamicActorDefinition {
	return definition instanceof DynamicActorDefinition;
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
