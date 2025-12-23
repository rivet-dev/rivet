import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import type { Logger } from "@/common/log";
import type { Registry } from "@/registry";
import type { Conn, ConnId } from "../../conn/mod";
import type { AnyDatabaseProvider, InferDatabaseClient } from "../../database";
import type { ActorDefinition, AnyActorDefinition } from "../../definition";
import type { ActorInstance, SaveStateOptions } from "../../instance/mod";
import type { Schedule } from "../../schedule";

/**
 * ActorContext class that provides access to actor methods and state
 */
export class ActorContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> {
	#actor: ActorInstance<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>;

	constructor(
		actor: ActorInstance<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) {
		this.#actor = actor;
	}

	/**
	 * Get the actor state
	 *
	 * @remarks
	 * This property is not available in `createState` since the state hasn't been created yet.
	 */
	get state(): TState extends never ? never : TState {
		return this.#actor.state as TState extends never ? never : TState;
	}

	/**
	 * Get the actor variables
	 *
	 * @remarks
	 * This property is not available in `createVars` since the variables haven't been created yet.
	 * Variables are only available if you define `vars` or `createVars` in your actor config.
	 */
	get vars(): TVars extends never ? never : TVars {
		return this.#actor.vars as TVars extends never ? never : TVars;
	}

	/**
	 * Broadcasts an event to all connected clients.
	 * @param name - The name of the event.
	 * @param args - The arguments to send with the event.
	 */
	broadcast<Args extends Array<unknown>>(name: string, ...args: Args): void {
		this.#actor.eventManager.broadcast(name, ...args);
		return;
	}

	/**
	 * Gets the logger instance.
	 */
	get log(): Logger {
		return this.#actor.log;
	}

	/**
	 * Gets actor ID.
	 */
	get actorId(): string {
		return this.#actor.id;
	}

	/**
	 * Gets the actor name.
	 */
	get name(): string {
		return this.#actor.name;
	}

	/**
	 * Gets the actor key.
	 */
	get key(): ActorKey {
		return this.#actor.key;
	}

	/**
	 * Gets the region.
	 */
	get region(): string {
		return this.#actor.region;
	}

	/**
	 * Gets the scheduler.
	 */
	get schedule(): Schedule {
		return this.#actor.schedule;
	}

	/**
	 * Gets the map of connections.
	 */
	get conns(): Map<
		ConnId,
		Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>
	> {
		return this.#actor.conns;
	}

	/**
	 * Returns the client for the given registry.
	 */
	client<R extends Registry<any>>(): Client<R> {
		return this.#actor.inlineClient as Client<R>;
	}

	/**
	 * Gets the database.
	 *
	 * @experimental
	 * @remarks
	 * This property is only available if you define a `db` provider in your actor config.
	 * @throws {DatabaseNotEnabled} If the database is not enabled.
	 */
	get db(): TDatabase extends never ? never : InferDatabaseClient<TDatabase> {
		return this.#actor.db as TDatabase extends never
			? never
			: InferDatabaseClient<TDatabase>;
	}

	/**
	 * Forces the state to get saved.
	 *
	 * @param opts - Options for saving the state.
	 */
	async saveState(opts: SaveStateOptions): Promise<void> {
		return this.#actor.stateManager.saveState(opts);
	}

	/**
	 * Prevents the actor from sleeping until promise is complete.
	 */
	waitUntil(promise: Promise<void>): void {
		this.#actor.waitUntil(promise);
	}

	/**
	 * AbortSignal that fires when the actor is stopping.
	 */
	get abortSignal(): AbortSignal {
		return this.#actor.abortSignal;
	}

	/**
	 * Forces the actor to sleep.
	 *
	 * Not supported on all drivers.
	 *
	 * @experimental
	 */
	sleep() {
		this.#actor.startSleep();
	}

	/**
	 * Forces the actor to destroy.
	 *
	 * This will return immediately, then call `onStop` and `onDestroy`.
	 *
	 * @experimental
	 */
	destroy() {
		this.#actor.startDestroy();
	}
}

export type ActorContextOf<AD extends AnyActorDefinition> =
	AD extends ActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		any
	>
		? ActorContext<S, CP, CS, V, I, DB>
		: never;
