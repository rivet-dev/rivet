const RIVET_EXTENSION = Symbol.for('@rivet-dev/flue/rivet-extension');
const FLUE_RIVET_ACTOR_CONFIG = Symbol.for('@rivet-dev/flue/actor-config');

export interface FlueRivetActorConfig {
	readonly [FLUE_RIVET_ACTOR_CONFIG]?: true;
	readonly [key: string | symbol]: unknown;
}

export interface FlueRivetActorDefinition<TConfig extends FlueRivetActorConfig = FlueRivetActorConfig> {
	readonly config: TConfig;
	readonly [key: string | symbol]: unknown;
}

export type RivetExtension<TDefinition extends FlueRivetActorDefinition = FlueRivetActorDefinition> = (
	base: TDefinition,
) => TDefinition;

interface BrandedRivetExtension extends RivetExtension<any> {
	readonly [RIVET_EXTENSION]: true;
}

export function markFlueRivetActorDefinition<TDefinition extends FlueRivetActorDefinition>(
	definition: TDefinition,
): TDefinition {
	if (!definition || typeof definition !== 'object' || !definition.config) {
		throw new Error('[flue] Rivet actor definitions must be objects with a config object.');
	}
	Object.defineProperty(definition.config, FLUE_RIVET_ACTOR_CONFIG, {
		value: true,
		enumerable: false,
		configurable: false,
	});
	return definition;
}

export function extend<TDefinition extends FlueRivetActorDefinition>(
	transform: RivetExtension<TDefinition>,
): RivetExtension<TDefinition> {
	if (typeof transform !== 'function') {
		throw new Error('[flue] extend() expects a function that returns a Rivet actor definition.');
	}
	Object.defineProperty(transform, RIVET_EXTENSION, {
		value: true,
		enumerable: false,
		configurable: false,
	});
	return transform;
}

export function resolveRivetExtension(
	mod: Record<string, unknown>,
	name: string,
	kind: 'Agent' | 'Workflow',
): RivetExtension {
	const extension = mod.rivet;
	if (extension === undefined) return identity;
	if (!isRivetExtension(extension)) {
		throw new Error(
			`[flue] ${kind} "${name}" rivet export must be created with extend((base) => definition) from "@rivet-dev/flue/extension".`,
		);
	}
	return (base) => assertFlueRivetActorDefinition(extension(base), name, kind);
}

export function assertFlueRivetActorDefinition(
	value: unknown,
	name: string,
	kind: 'Agent' | 'Workflow',
): FlueRivetActorDefinition {
	if (
		!value ||
		typeof value !== 'object' ||
		!('config' in value) ||
		typeof (value as { config?: unknown }).config !== 'object' ||
		(value as { config?: FlueRivetActorConfig }).config?.[FLUE_RIVET_ACTOR_CONFIG] !== true
	) {
		throw new Error(
			`[flue] ${kind} "${name}" rivet extension must return the received Flue actor definition or a wrapped definition that preserves definition.config.`,
		);
	}
	return value as FlueRivetActorDefinition;
}

function identity<T extends FlueRivetActorDefinition>(definition: T): T {
	return definition;
}

function isRivetExtension(value: unknown): value is BrandedRivetExtension {
	return (
		typeof value === 'function' &&
		RIVET_EXTENSION in value &&
		(value as BrandedRivetExtension)[RIVET_EXTENSION] === true
	);
}
