import type { ActorKey } from "@/actor/mod";
import type { ActorConfig, GlobalActorOptionsInput } from "@/actor/config";
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

abstract class DynamicActorContextBase<TInput = unknown> {
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

export class DynamicActorLoaderContext<
	TInput = unknown,
> extends DynamicActorContextBase<TInput> {}

export class DynamicActorAuthContext<
	TInput = unknown,
> extends DynamicActorContextBase<TInput> {
	readonly request: Request | undefined;

	constructor(
		inlineClient: AnyClient,
		actorId: string,
		name: string,
		key: ActorKey,
		input: TInput,
		region: string,
		request: Request | undefined,
	) {
		super(inlineClient, actorId, name, key, input, region);
		this.request = request;
	}
}

export type DynamicActorLoader<TInput = unknown> = (
	context: DynamicActorLoaderContext<TInput>,
) => Promise<DynamicActorLoadResult> | DynamicActorLoadResult;

export type DynamicActorAuth<TConnParams = unknown, TInput = unknown> = (
	context: DynamicActorAuthContext<TInput>,
	params: TConnParams,
) => Promise<void> | void;

export interface DynamicActorOptionsInput {
	options?: GlobalActorOptionsInput;
}

export interface DynamicActorConfigInput<
	TInput = unknown,
	TConnParams = unknown,
> extends DynamicActorOptionsInput {
	load: DynamicActorLoader<TInput>;
	auth?: DynamicActorAuth<TConnParams, TInput>;
}

export class DynamicActorDefinition<TInput = unknown, TConnParams = unknown>
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
	#auth: DynamicActorAuth<TConnParams, TInput> | undefined;
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

	constructor(input: DynamicActorConfigInput<TInput, TConnParams>) {
		this.#loader = input.load;
		this.#auth = input.auth;
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

	get auth(): DynamicActorAuth<TConnParams, TInput> | undefined {
		return this.#auth;
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
): definition is DynamicActorDefinition<any, any> {
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

export function createDynamicActorAuthContext<TInput>(
	inlineClient: AnyClient,
	actorId: string,
	name: string,
	key: ActorKey,
	input: TInput,
	region: string,
	request: Request | undefined,
): DynamicActorAuthContext<TInput> {
	return new DynamicActorAuthContext(
		inlineClient,
		actorId,
		name,
		key,
		input,
		region,
		request,
	);
}
