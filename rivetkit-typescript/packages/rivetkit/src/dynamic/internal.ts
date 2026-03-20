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

export class DynamicActorLoaderContext<TInput = unknown> extends DynamicActorContextBase<TInput> {}

export class DynamicActorAuthContext<TInput = unknown> extends DynamicActorContextBase<TInput> {
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

export interface DynamicStartupOptions {
	/** Maximum time in milliseconds to wait for actor startup before timing out. */
	timeoutMs?: number;
	/** Initial delay in milliseconds before the first retry attempt. */
	retryInitialDelayMs?: number;
	/** Maximum delay in milliseconds between retry attempts. */
	retryMaxDelayMs?: number;
	/** Multiplier applied to the delay after each retry attempt. */
	retryMultiplier?: number;
	/** Whether to add random jitter to retry delays. */
	retryJitter?: boolean;
	/** Maximum number of retry attempts before giving up. Set to 0 for unlimited. */
	maxAttempts?: number;
}

export const DYNAMIC_STARTUP_DEFAULTS = {
	timeoutMs: 15_000,
	retryInitialDelayMs: 1_000,
	retryMaxDelayMs: 30_000,
	retryMultiplier: 2,
	retryJitter: true,
	maxAttempts: 20,
} as const satisfies Required<DynamicStartupOptions>;

export type DynamicActorOptions = GlobalActorOptionsInput & {
	startup?: DynamicStartupOptions;
};

export class DynamicActorReloadContext<TInput = unknown> extends DynamicActorContextBase<TInput> {
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

export type DynamicActorCanReload<TInput = unknown> = (
	context: DynamicActorReloadContext<TInput>,
) => Promise<boolean> | boolean;

export interface DynamicActorOptionsInput {
	options?: DynamicActorOptions;
}

export interface DynamicActorConfigInput<
	TInput = unknown,
	TConnParams = unknown,
> extends DynamicActorOptionsInput {
	load: DynamicActorLoader<TInput>;
	auth?: DynamicActorAuth<TConnParams, TInput>;
	canReload?: DynamicActorCanReload<TInput>;
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
	#canReload: DynamicActorCanReload<TInput> | undefined;
	#startupOptions: Required<DynamicStartupOptions>;
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
		this.#canReload = input.canReload;
		this.#startupOptions = {
			...DYNAMIC_STARTUP_DEFAULTS,
			...input.options?.startup,
		};
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

	get canReload(): DynamicActorCanReload<TInput> | undefined {
		return this.#canReload;
	}

	get startupOptions(): Required<DynamicStartupOptions> {
		return this.#startupOptions;
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

export function createDynamicActorReloadContext<TInput>(
	inlineClient: AnyClient,
	actorId: string,
	name: string,
	key: ActorKey,
	input: TInput,
	region: string,
	request: Request | undefined,
): DynamicActorReloadContext<TInput> {
	return new DynamicActorReloadContext(
		inlineClient,
		actorId,
		name,
		key,
		input,
		region,
		request,
	);
}
