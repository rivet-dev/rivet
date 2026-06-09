import type {
	AnyActorDefinition,
	BaseActorDefinition,
} from "@/actor/definition";
import type { AnyClient } from "@/client/client";

export const DYNAMIC_ACTOR_DEFINITION_SYMBOL = Symbol.for(
	"rivetkit.dynamic_actor_definition",
);

/** Source resolved by a dynamic actor loader. */
export interface DynamicActorLoadResult {
	/** Actor source text. Must export an actor definition as default export. */
	source: string;
	/** Module format of the source text. Defaults to esm-js. */
	sourceFormat?: "esm-js" | "commonjs-js";
	/** Worker thread resource configuration for this actor instance. */
	worker?: {
		/** Maximum old-generation heap size for the worker in MiB. */
		memoryLimitMb?: number;
	};
}

export interface DynamicActorLoadContext {
	/** Actor key for the instance being started. */
	key: string[];
	/** Inline client connected to the same registry. */
	client(): Promise<AnyClient>;
}

export type DynamicActorLoader = (
	c: DynamicActorLoadContext,
) => DynamicActorLoadResult | Promise<DynamicActorLoadResult>;

/**
 * Static options for a dynamic actor. Unlike regular actors, the loaded
 * definition is unknown until the actor starts, so anything the runtime needs
 * at registry-build time (timeouts, sleep behavior, database usage) must be
 * declared here.
 */
export interface DynamicActorOptions {
	/** Display name for the actor in the Inspector UI. */
	name?: string;
	/** Icon for the actor in the Inspector UI. */
	icon?: string;
	/** Enables SQLite for the loaded actor. Defaults to false. */
	database?: boolean;
	/** Allows the loaded actor to hibernate raw WebSockets. Defaults to false. */
	canHibernateWebSocket?: boolean;
	actionTimeout?: number;
	sleepTimeout?: number;
	sleepGracePeriod?: number;
	noSleep?: boolean;
	maxQueueSize?: number;
	maxQueueMessageSize?: number;
}

export interface DynamicActorDefinition extends AnyActorDefinition {
	readonly [DYNAMIC_ACTOR_DEFINITION_SYMBOL]: true;
	readonly loader: DynamicActorLoader;
	readonly options: DynamicActorOptions;
}

export function isDynamicActorDefinition(
	definition:
		| AnyActorDefinition
		| BaseActorDefinition<any, any, any, any, any, any, any, any, any>,
): definition is DynamicActorDefinition {
	return (
		(definition as Partial<DynamicActorDefinition>)[
			DYNAMIC_ACTOR_DEFINITION_SYMBOL
		] === true
	);
}
