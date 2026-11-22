/**
 * Singleton registrations for one cluster, keyed by singleton name.
 *
 * @internal
 */
import type * as Context from "effect/Context";
import type * as Effect from "effect/Effect";
import type * as Scope from "effect/Scope";

/** @internal */
export interface SingletonRegistration {
	readonly run: Effect.Effect<void, unknown, Scope.Scope>;
	readonly context: Context.Context<never>;
}

/** @internal */
export interface SingletonRegistry {
	readonly get: (name: string) => SingletonRegistration | undefined;
	readonly names: () => ReadonlyArray<string>;
	/** Returns `false` when the name already has a registration. */
	readonly register: (
		name: string,
		registration: SingletonRegistration,
	) => boolean;
	readonly unregister: (
		name: string,
		registration: SingletonRegistration,
	) => void;
}

/** @internal */
export const makeSingletonRegistry = (): SingletonRegistry => {
	const registrations = new Map<string, SingletonRegistration>();
	return {
		get: (name) => registrations.get(name),
		names: () => Array.from(registrations.keys()),
		register: (name, registration) => {
			if (registrations.has(name)) return false;
			registrations.set(name, registration);
			return true;
		},
		unregister: (name, registration) => {
			if (registrations.get(name) === registration)
				registrations.delete(name);
		},
	};
};
