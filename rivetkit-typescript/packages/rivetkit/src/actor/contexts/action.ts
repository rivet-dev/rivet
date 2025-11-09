import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import type { Logger } from "@/common/log";
import type { Registry } from "@/registry/mod";
import type { Conn, ConnId } from "../conn/mod";
import type { AnyDatabaseProvider, InferDatabaseClient } from "../database";
import type { SaveStateOptions } from "../instance/state-manager";
import type { Schedule } from "../schedule";
import type { ActorContext } from "./actor";

/**
 * Context for a remote procedure call.
 *
 * @typeParam A Actor this action belongs to
 */
export class ActionContext<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> {
	#actorContext: ActorContext<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>;

	/**
	 * Should not be called directly.
	 *
	 * @param actorContext - The actor context
	 * @param conn - The connection associated with the action
	 */
	constructor(
		actorContext: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		public readonly conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) {
		this.#actorContext = actorContext;
	}

	/**
	 * Get the actor state
	 */
	get state(): TState {
		return this.#actorContext.state;
	}

	/**
	 * Get the actor variables
	 */
	get vars(): TVars {
		return this.#actorContext.vars;
	}

	/**
	 * Broadcasts an event to all connected clients.
	 */
	broadcast(name: string, ...args: any[]): void {
		this.#actorContext.broadcast(name, ...args);
	}

	/**
	 * Gets the logger instance.
	 */
	get log(): Logger {
		return this.#actorContext.log;
	}

	/**
	 * Gets actor ID.
	 */
	get actorId(): string {
		return this.#actorContext.actorId;
	}

	/**
	 * Gets the actor name.
	 */
	get name(): string {
		return this.#actorContext.name;
	}

	/**
	 * Gets the actor key.
	 */
	get key(): ActorKey {
		return this.#actorContext.key;
	}

	/**
	 * Gets the region.
	 */
	get region(): string {
		return this.#actorContext.region;
	}

	/**
	 * Gets the scheduler.
	 */
	get schedule(): Schedule {
		return this.#actorContext.schedule;
	}

	/**
	 * Gets the map of connections.
	 */
	get conns(): Map<
		ConnId,
		Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>
	> {
		return this.#actorContext.conns;
	}

	/**
	 * Returns the client for the given registry.
	 */
	client<R extends Registry<any>>(): Client<R> {
		return this.#actorContext.client<R>();
	}

	/**
	 * @experimental
	 */
	get db(): InferDatabaseClient<TDatabase> {
		return this.#actorContext.db;
	}

	/**
	 * Forces the state to get saved.
	 */
	async saveState(opts: SaveStateOptions): Promise<void> {
		return this.#actorContext.saveState(opts);
	}

	/**
	 * Prevents the actor from sleeping until promise is complete.
	 */
	waitUntil(promise: Promise<void>): void {
		this.#actorContext.waitUntil(promise);
	}

	/**
	 * AbortSignal that fires when the actor is stopping.
	 */
	get abortSignal(): AbortSignal {
		return this.#actorContext.abortSignal;
	}

	/**
	 * Forces the actor to sleep.
	 *
	 * Not supported on all drivers.
	 *
	 * @experimental
	 */
	sleep() {
		this.#actorContext.sleep();
	}
}
