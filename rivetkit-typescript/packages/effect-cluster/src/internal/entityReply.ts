/**
 * Delivery of scheduled-ask replies back to the asking entity.
 *
 * Portions adapted from Effect (MIT License,
 * Copyright (c) 2023 Effectful Technologies Inc). See `LICENSE-effect`.
 *
 * @internal
 */
import * as Context from "effect/Context";
import * as Deferred from "effect/Deferred";
import * as Effect from "effect/Effect";

/** @internal */
export class CurrentEntityName extends Context.Service<
	CurrentEntityName,
	string
>()("@rivetkit/effect-cluster/CurrentEntityName") {}

/**
 * Completes the delayed asks made by one entity actor's handlers. The
 * destination actor pushes the stored reply back to the caller's
 * `deliverReply` action, which finds the waiting `Deferred` here.
 *
 * @internal
 */
export interface EntityReplyRegistry {
	readonly register: (
		requestId: string,
		waiter: Deferred.Deferred<string>,
	) => void;
	readonly unregister: (
		requestId: string,
		waiter: Deferred.Deferred<string>,
	) => void;
	readonly deliver: (requestId: string, reply: string) => boolean;
}

/** @internal */
export const makeReplyRegistry = (): EntityReplyRegistry => {
	const waiters = new Map<string, Set<Deferred.Deferred<string>>>();
	return {
		register(requestId, waiter) {
			const registered = waiters.get(requestId) ?? new Set();
			registered.add(waiter);
			waiters.set(requestId, registered);
		},
		unregister(requestId, waiter) {
			const registered = waiters.get(requestId);
			if (registered === undefined) return;
			registered.delete(waiter);
			if (registered.size === 0) waiters.delete(requestId);
		},
		deliver(requestId, reply) {
			const registered = waiters.get(requestId);
			if (registered === undefined) return false;
			waiters.delete(requestId);
			for (const waiter of registered) {
				Deferred.doneUnsafe(waiter, Effect.succeed(reply));
			}
			return true;
		},
	};
};

/** @internal */
export class CurrentReplyRegistry extends Context.Service<
	CurrentReplyRegistry,
	EntityReplyRegistry
>()("@rivetkit/effect-cluster/CurrentReplyRegistry") {}
