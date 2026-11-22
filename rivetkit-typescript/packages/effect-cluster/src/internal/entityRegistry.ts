/**
 * Entity handler registrations for one cluster, keyed by entity type.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */

import type * as Context from "effect/Context";
import type * as Entity from "effect/cluster/Entity";
import type * as Effect from "effect/Effect";

/** @internal */
export interface EntityRegistration {
	readonly entity: Entity.Entity<any, any>;
	readonly build: Effect.Effect<
		Record<string, (request: any) => any>,
		never,
		never
	>;
	readonly options:
		| {
				readonly concurrency?: number | "unbounded" | undefined;
				readonly disableFatalDefects?: boolean | undefined;
				readonly defectRetryPolicy?: unknown;
				readonly spanAttributes?: Record<string, string> | undefined;
				readonly mailboxCapacity?: number | "unbounded" | undefined;
		  }
		| undefined;
	readonly context: Context.Context<never>;
}

/** @internal */
export interface EntityRegistry {
	readonly get: (type: string) => EntityRegistration | undefined;
	/** Returns `false` when the type already has a registration. */
	readonly register: (
		type: string,
		registration: EntityRegistration,
	) => boolean;
	readonly unregister: (
		type: string,
		registration: EntityRegistration,
	) => void;
}

/** @internal */
export const makeEntityRegistry = (): EntityRegistry => {
	const registrations = new Map<string, EntityRegistration>();
	return {
		get: (type) => registrations.get(type),
		register: (type, registration) => {
			if (registrations.has(type)) return false;
			registrations.set(type, registration);
			return true;
		},
		unregister: (type, registration) => {
			if (registrations.get(type) === registration)
				registrations.delete(type);
		},
	};
};
