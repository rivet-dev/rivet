import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import type { Logger } from "@/common/log";
import type { Registry } from "@/registry";
import type { Conn, ConnId } from "../../conn/mod";
import type { AnyDatabaseProvider, InferDatabaseClient } from "../../database";
import type { ActorDefinition, AnyActorDefinition } from "../../definition";
import * as errors from "../../errors";
import { ActorKv } from "../../instance/kv";
import type {
	ActorInstance,
	AnyActorInstance,
	SaveStateOptions,
} from "../../instance/mod";
import { ActorQueue } from "../../instance/queue";
import type { Schedule } from "../../schedule";
import {
	type EventSchemaConfig,
	type InferEventArgs,
	type InferSchemaMap,
	type QueueSchemaConfig,
	hasSchemaConfigKey,
	validateSchemaSync,
} from "../../schema";

export const ACTOR_CONTEXT_INTERNAL_SYMBOL = Symbol.for(
	"rivetkit.actorContextInternal",
);

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
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
	[ACTOR_CONTEXT_INTERNAL_SYMBOL]!: AnyActorInstance;
	#actor: ActorInstance<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	#kv: ActorKv | undefined;
	#queue:
		| ActorQueue<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
		  >
		| undefined;

	constructor(
		actor: ActorInstance<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) {
		this.#actor = actor;
		this[ACTOR_CONTEXT_INTERNAL_SYMBOL] = actor as AnyActorInstance;
	}

	/**
	 * Gets the KV storage interface.
	 */
	get kv(): ActorKv {
		if (!this.#kv) {
			this.#kv = new ActorKv(this.#actor.driver, this.#actor.id);
		}
		return this.#kv;
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
	broadcast<K extends keyof TEvents & string>(
		name: K,
		...args: InferEventArgs<InferSchemaMap<TEvents>[K]>
	): void;
	broadcast(
		name: keyof TEvents extends never ? string : never,
		...args: Array<unknown>
	): void;
	broadcast(name: string, ...args: Array<unknown>): void {
		if (
			this.#actor.config.events !== undefined &&
			!hasSchemaConfigKey(this.#actor.config.events, name)
		) {
			this.#actor.rLog.warn({
				msg: "broadcasting event not defined in actor events config",
				eventName: name,
			});
		}

		const payload = args.length === 1 ? args[0] : args;
		const result = validateSchemaSync(
			this.#actor.config.events,
			name as keyof TEvents & string,
			payload,
		);
		if (!result.success) {
			throw new errors.EventPayloadInvalid(name, result.issues);
		}
		if (args.length === 1) {
			this.#actor.eventManager.broadcast(name, result.data);
			return;
		}
		if (Array.isArray(result.data)) {
			this.#actor.eventManager.broadcast(
				name,
				...(result.data as unknown[]),
			);
			return;
		}
		this.#actor.eventManager.broadcast(name, ...args);
	}

	/**
	 * Gets the logger instance.
	 */
	get log(): Logger {
		return this.#actor.log;
	}

	/**
	 * Access to queue receive helpers.
	 */
	get queue(): ActorQueue<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> {
		if (!this.#queue) {
			this.#queue = new ActorQueue(
				this.#actor.queueManager,
				this.#actor.abortSignal,
			);
		}
		return this.#queue;
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
		Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>
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
	 * Prevents the actor from sleeping while the given promise is running.
	 *
	 * Use this when performing async operations in the `run` handler or other
	 * background contexts where you need to ensure the actor stays awake.
	 *
	 * Returns the resolved value and resets the sleep timer on completion.
	 * Errors are propagated to the caller.
	 */
	keepAwake<T>(promise: Promise<T>): Promise<T> {
		return this.#actor.keepAwake(promise);
	}

	/**
	 * AbortSignal that fires when the actor is stopping.
	 */
	get abortSignal(): AbortSignal {
		return this.#actor.abortSignal;
	}

	/**
	 * True when the actor is stopping.
	 *
	 * Alias for `c.abortSignal.aborted`.
	 */
	get aborted(): boolean {
		return this.#actor.abortSignal.aborted;
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
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ActorContext<S, CP, CS, V, I, DB, E, Q>
		: never;
