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
	 * Defaults to `esm-js`.
	 */
	sourceFormat?: DynamicSourceFormat;
	nodeProcess?: DynamicNodeProcessConfig;
}

export class DynamicActorLoaderContext<TInput = unknown> {
	readonly actorId: string;
	readonly name: string;
	readonly key: ActorKey;
	readonly input: TInput;
	readonly region: string;
	#inlineClient: AnyClient;

	constructor(
		inlineClient: AnyClient,
		actorId: string,
		name: string,
		key: ActorKey,
		input: TInput,
		region: string,
	) {
		this.#inlineClient = inlineClient;
		this.actorId = actorId;
		this.name = name;
		this.key = key;
		this.input = input;
		this.region = region;
	}

	client<R extends Registry<any>>(): Client<R> {
		return this.#inlineClient as Client<R>;
	}
}

export type DynamicActorLoader<TInput = unknown> = (
	context: DynamicActorLoaderContext<TInput>,
) => Promise<DynamicActorLoadResult> | DynamicActorLoadResult;

export interface DynamicActorConfigInput {
	options?: GlobalActorOptionsInput;
}

export class DynamicActorDefinition<TInput = unknown>
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
	#loader: DynamicActorLoader<TInput>;
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

	constructor(
		loader: DynamicActorLoader<TInput>,
		input: DynamicActorConfigInput = {},
	) {
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

	get loader(): DynamicActorLoader<TInput> {
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
): definition is DynamicActorDefinition<any> {
	return definition instanceof DynamicActorDefinition;
}

export function createDynamicActorLoaderContext<TInput>(
	inlineClient: AnyClient,
	actorId: string,
	name: string,
	key: ActorKey,
	input: TInput,
	region: string,
): DynamicActorLoaderContext<TInput> {
	return new DynamicActorLoaderContext(
		inlineClient,
		actorId,
		name,
		key,
		input,
		region,
	);
}
