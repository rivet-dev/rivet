import type {
	AnyActorDefinition,
	BaseActorDefinition,
} from "@/actor/definition";

export const DYNAMIC_ACTOR_DEFINITION_SYMBOL = Symbol.for(
	"rivetkit.dynamic_actor_definition",
);

export interface DynamicActorSource {
	source: string;
	nodeProcess?: {
		memoryLimit?: number;
		cpuTimeLimitMs?: number;
	};
}

export interface DynamicActorLoadContext {
	key: string[];
	client(): Promise<any>;
}

export type DynamicActorLoader = (
	c: DynamicActorLoadContext,
) => DynamicActorSource | Promise<DynamicActorSource>;

export interface DynamicActorDefinition extends AnyActorDefinition {
	readonly [DYNAMIC_ACTOR_DEFINITION_SYMBOL]: true;
	readonly loader: DynamicActorLoader;
}

export function isDynamicActorDefinition(
	definition: BaseActorDefinition<
		any,
		any,
		any,
		any,
		any,
		any,
		any,
		any,
		any
	>,
): definition is DynamicActorDefinition {
	return (
		(definition as Partial<DynamicActorDefinition>)[
			DYNAMIC_ACTOR_DEFINITION_SYMBOL
		] === true
	);
}
