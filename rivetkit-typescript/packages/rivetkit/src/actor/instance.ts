import * as cbor from "cbor-x";
import invariant from "invariant";
import onChange from "on-change";
import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import { getBaseLogger, getIncludeTarget, type Logger } from "@/common/log";
import { isCborSerializable, stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { ActorInspector } from "@/inspector/actor";
import type { Registry } from "@/mod";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import type * as protocol from "@/schemas/client-protocol/mod";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import {
	arrayBuffersEqual,
	bufferToArrayBuffer,
	EXTRA_ERROR_LOG,
	idToStr,
	promiseWithResolvers,
	SinglePromiseQueue,
} from "@/utils";
import { ActionContext } from "./action";
import type { ActorConfig, OnConnectOptions } from "./config";
import { Conn, type ConnId, generateConnRequestId } from "./conn";
import {
	CONN_DRIVERS,
	ConnDriverKind,
	getConnDriverKindFromState,
} from "./conn-drivers";
import type { ConnSocket } from "./conn-socket";
import { ActorContext } from "./context";
import type { AnyDatabaseProvider, InferDatabaseClient } from "./database";
import type { ActorDriver } from "./driver";
import * as errors from "./errors";
import { serializeActorKey } from "./keys";
import { KEYS, makeConnKey } from "./kv";
import type {
	PersistedActor,
	PersistedConn,
	PersistedHibernatableConn,
	PersistedScheduleEvent,
} from "./persisted";
import { processMessage } from "./protocol/old";
import { CachedSerializer } from "./protocol/serde";
import { Schedule } from "./schedule";
import { DeadlineError, deadline, isConnStatePath, isStatePath } from "./utils";

export const PERSIST_SYMBOL = Symbol("persist");

/**
 * Options for the `_saveState` method.
 */
export interface SaveStateOptions {
	/**
	 * Forces the state to be saved immediately. This function will return when the state has saved successfully.
	 */
	immediate?: boolean;
	/** Bypass ready check for stopping. */
	allowStoppingState?: boolean;
}

/** Actor type alias with all `any` types. Used for `extends` in classes referencing this actor. */
export type AnyActorInstance = ActorInstance<
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any,
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any,
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any,
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any,
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any,
	// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
	any
>;

export type ExtractActorState<A extends AnyActorInstance> =
	A extends ActorInstance<
		infer State,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any
	>
		? State
		: never;

export type ExtractActorConnParams<A extends AnyActorInstance> =
	A extends ActorInstance<
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		infer ConnParams,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any
	>
		? ConnParams
		: never;

export type ExtractActorConnState<A extends AnyActorInstance> =
	A extends ActorInstance<
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		infer ConnState,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any,
		// biome-ignore lint/suspicious/noExplicitAny: Must be used for `extends`
		any
	>
		? ConnState
		: never;

enum CanSleep {
	Yes,
	NotReady,
	ActiveConns,
	ActiveHonoHttpRequests,
	ActiveRawWebSockets,
}

export class ActorInstance<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	// Shared actor context for this instance
	actorContext: ActorContext<S, CP, CS, V, I, DB>;

	/** Actor log, intended for the user to call */
	#log!: Logger;

	get log(): Logger {
		invariant(this.#log, "log not configured");
		return this.#log;
	}

	/** Runtime log, intended for internal actor logs */
	#rLog!: Logger;

	get rLog(): Logger {
		invariant(this.#rLog, "log not configured");
		return this.#rLog;
	}

	#sleepCalled = false;
	#stopCalled = false;

	get isStopping() {
		return this.#stopCalled;
	}

	#persistChanged = false;
	#isInOnStateChange = false;

	/**
	 * The proxied state that notifies of changes automatically.
	 *
	 * Any data that should be stored indefinitely should be held within this object.
	 */
	#persist!: PersistedActor<S, CP, CS, I>;

	get [PERSIST_SYMBOL](): PersistedActor<S, CP, CS, I> {
		return this.#persist;
	}

	/** Raw state without the proxy wrapper */
	#persistRaw!: PersistedActor<S, CP, CS, I>;

	#persistWriteQueue = new SinglePromiseQueue();
	#alarmWriteQueue = new SinglePromiseQueue();

	#lastSaveTime = 0;
	#pendingSaveTimeout?: NodeJS.Timeout;

	#vars?: V;

	#backgroundPromises: Promise<void>[] = [];

	#abortController = new AbortController();

	#config: ActorConfig<S, CP, CS, V, I, DB>;
	#actorDriver!: ActorDriver;
	#inlineClient!: Client<Registry<any>>;
	#actorId!: string;

	#name!: string;

	get name(): string {
		return this.#name;
	}

	#key!: ActorKey;

	get key(): ActorKey {
		return this.#key;
	}

	#region!: string;

	get region(): string {
		return this.#region;
	}

	#ready = false;

	#connections = new Map<ConnId, Conn<S, CP, CS, V, I, DB>>();

	get conns(): Map<ConnId, Conn<S, CP, CS, V, I, DB>> {
		return this.#connections;
	}

	#subscriptionIndex = new Map<string, Set<Conn<S, CP, CS, V, I, DB>>>();
	#changedConnections = new Set<ConnId>();

	#sleepTimeout?: NodeJS.Timeout;

	/**
	 * Track active HTTP requests through Hono router so sleep logic can
	 * account for them. Does not include WebSockets.
	 **/
	#activeHonoHttpRequests = 0;
	#activeRawWebSockets = new Set<UniversalWebSocket>();

	#schedule!: Schedule;

	get schedule(): Schedule {
		return this.#schedule;
	}

	#db!: InferDatabaseClient<DB>;

	/**
	 * Gets the database.
	 * @experimental
	 */
	get db(): InferDatabaseClient<DB> {
		if (!this.#db) {
			throw new errors.DatabaseNotEnabled();
		}
		return this.#db;
	}

	#inspector = new ActorInspector(() => {
		return {
			isDbEnabled: async () => {
				return this.#db !== undefined;
			},
			getDb: async () => {
				return this.db;
			},
			isStateEnabled: async () => {
				return this.stateEnabled;
			},
			getState: async () => {
				this.#validateStateEnabled();

				// Must return from `#persistRaw` in order to not return the `onchange` proxy
				return this.#persistRaw.state as Record<string, any> as unknown;
			},
			getRpcs: async () => {
				return Object.keys(this.#config.actions);
			},
			getConnections: async () => {
				return Array.from(this.#connections.entries()).map(
					([id, conn]) => ({
						id,
						params: conn.params as any,
						state: conn.__stateEnabled ? conn.state : undefined,
						subscriptions: conn.subscriptions.size,
						lastSeen: conn.lastSeen,
						stateEnabled: conn.__stateEnabled,
						isHibernatable: conn.isHibernatable,
						hibernatableRequestId: conn.__persist
							.hibernatableRequestId
							? idToStr(conn.__persist.hibernatableRequestId)
							: undefined,
						driver: conn.__driverState
							? getConnDriverKindFromState(conn.__driverState)
							: undefined,
					}),
				);
			},
			setState: async (state: unknown) => {
				this.#validateStateEnabled();

				// Must set on `#persist` instead of `#persistRaw` in order to ensure that the `Proxy` is correctly configured
				//
				// We have to use `...` so `on-change` recognizes the changes to `state` (i.e. set #persistChanged` to true). This is because:
				// 1. In `getState`, we returned the value from `persistRaw`, which does not have the Proxy to monitor state changes
				// 2. If we were to assign `state` to `#persist.s`, `on-change` would assume nothing changed since `state` is still === `#persist.s` since we returned a reference in `getState`
				this.#persist.state = { ...(state as S) };
				await this.saveState({ immediate: true });
			},
			executeAction: async (name, params) => {
				const requestId = generateConnRequestId();
				const conn = await this.createConn(
					{
						requestId: requestId,
						hibernatable: false,
						driverState: { [ConnDriverKind.HTTP]: {} },
					},
					undefined,
					undefined,
				);

				try {
					return await this.executeAction(
						new ActionContext(this.actorContext, conn),
						name,
						params || [],
					);
				} finally {
					this.__connDisconnected(conn, true, requestId);
				}
			},
		};
	});

	get id() {
		return this.#actorId;
	}

	get inlineClient(): Client<Registry<any>> {
		return this.#inlineClient;
	}

	get inspector() {
		return this.#inspector;
	}

	get #sleepingSupported(): boolean {
		return this.#actorDriver.startSleep !== undefined;
	}

	/**
	 * This constructor should never be used directly.
	 *
	 * Constructed in {@link ActorInstance.start}.
	 *
	 * @private
	 */
	constructor(config: ActorConfig<S, CP, CS, V, I, DB>) {
		this.#config = config;
		this.actorContext = new ActorContext(this);
	}

	// MARK: Initialization
	async start(
		actorDriver: ActorDriver,
		inlineClient: Client<Registry<any>>,
		actorId: string,
		name: string,
		key: ActorKey,
		region: string,
	) {
		const logParams = {
			actor: name,
			key: serializeActorKey(key),
			actorId,
		};

		const extraLogParams = actorDriver.getExtraActorLogParams?.();
		if (extraLogParams) Object.assign(logParams, extraLogParams);

		this.#log = getBaseLogger().child(
			Object.assign(
				getIncludeTarget() ? { target: "actor" } : {},
				logParams,
			),
		);
		this.#rLog = getBaseLogger().child(
			Object.assign(
				getIncludeTarget() ? { target: "actor-runtime" } : {},
				logParams,
			),
		);

		this.#actorDriver = actorDriver;
		this.#inlineClient = inlineClient;
		this.#actorId = actorId;
		this.#name = name;
		this.#key = key;
		this.#region = region;
		this.#schedule = new Schedule(this);

		// Read initial state from KV storage
		const [persistDataBuffer] = await this.#actorDriver.kvBatchGet(
			this.#actorId,
			[KEYS.PERSIST_DATA],
		);
		invariant(
			persistDataBuffer !== null,
			"persist data has not been set, it should be set when initialized",
		);
		const bareData =
			ACTOR_VERSIONED.deserializeWithEmbeddedVersion(persistDataBuffer);
		const persistData = this.#convertFromBarePersisted(bareData);

		if (persistData.hasInitialized) {
			// List all connection keys
			const connEntries = await this.#actorDriver.kvListPrefix(
				this.#actorId,
				KEYS.CONN_PREFIX,
			);

			// Decode connections
			const connections: PersistedConn<CP, CS>[] = [];
			for (const [_key, value] of connEntries) {
				try {
					const conn = cbor.decode(value) as PersistedConn<CP, CS>;
					connections.push(conn);
				} catch (error) {
					this.#rLog.error({
						msg: "failed to decode connection",
						error: stringifyError(error),
					});
				}
			}

			this.#rLog.info({
				msg: "actor restoring",
				connections: connections.length,
				hibernatableWebSockets: persistData.hibernatableConns.length,
			});

			// Set initial state
			this.#initPersistProxy(persistData);

			// Create connection instances
			for (const connPersist of connections) {
				// Create connections
				const conn = new Conn<S, CP, CS, V, I, DB>(this, connPersist);
				this.#connections.set(conn.id, conn);

				// Register event subscriptions
				for (const sub of connPersist.subscriptions) {
					this.#addSubscription(sub.eventName, conn, true);
				}
			}
		} else {
			this.#rLog.info({ msg: "actor creating" });

			// Initialize actor state
			let stateData: unknown;
			if (this.stateEnabled) {
				this.#rLog.info({ msg: "actor state initializing" });

				if ("createState" in this.#config) {
					this.#config.createState;

					// Convert state to undefined since state is not defined yet here
					stateData = await this.#config.createState(
						this.actorContext as unknown as ActorContext<
							undefined,
							undefined,
							undefined,
							undefined,
							undefined,
							undefined
						>,
						persistData.input!,
					);
				} else if ("state" in this.#config) {
					stateData = structuredClone(this.#config.state);
				} else {
					throw new Error(
						"Both 'createState' or 'state' were not defined",
					);
				}
			} else {
				this.#rLog.debug({ msg: "state not enabled" });
			}

			// Save state and mark as initialized
			persistData.state = stateData as S;
			persistData.hasInitialized = true;

			// Update state
			this.#rLog.debug({ msg: "writing state" });
			const bareData = this.#convertToBarePersisted(persistData);
			await this.#actorDriver.kvBatchPut(this.#actorId, [
				[
					KEYS.PERSIST_DATA,
					ACTOR_VERSIONED.serializeWithEmbeddedVersion(bareData),
				],
			]);

			this.#initPersistProxy(persistData);

			// Notify creation
			if (this.#config.onCreate) {
				await this.#config.onCreate(
					this.actorContext,
					persistData.input!,
				);
			}
		}

		// TODO: Exit process if this errors
		if (this.#varsEnabled) {
			let vars: V | undefined;
			if ("createVars" in this.#config) {
				const dataOrPromise = this.#config.createVars(
					this.actorContext as unknown as ActorContext<
						undefined,
						undefined,
						undefined,
						undefined,
						undefined,
						any
					>,
					this.#actorDriver.getContext(this.#actorId),
				);
				if (dataOrPromise instanceof Promise) {
					vars = await deadline(
						dataOrPromise,
						this.#config.options.createVarsTimeout,
					);
				} else {
					vars = dataOrPromise;
				}
			} else if ("vars" in this.#config) {
				vars = structuredClone(this.#config.vars);
			} else {
				throw new Error(
					"Could not variables from 'createVars' or 'vars'",
				);
			}
			this.#vars = vars;
		}

		// TODO: Exit process if this errors
		this.#rLog.info({ msg: "actor starting" });
		if (this.#config.onStart) {
			const result = this.#config.onStart(this.actorContext);
			if (result instanceof Promise) {
				await result;
			}
		}

		// Setup Database
		if ("db" in this.#config && this.#config.db) {
			const client = await this.#config.db.createClient({
				getDatabase: () => actorDriver.getDatabase(this.#actorId),
			});
			this.#rLog.info({ msg: "database migration starting" });
			await this.#config.db.onMigrate?.(client);
			this.#rLog.info({ msg: "database migration complete" });
			this.#db = client;
		}

		// Set alarm for next scheduled event if any exist after finishing initiation sequence
		if (this.#persist.scheduledEvents.length > 0) {
			await this.#queueSetAlarm(
				this.#persist.scheduledEvents[0].timestamp,
			);
		}

		this.#rLog.info({ msg: "actor ready" });
		this.#ready = true;

		// Must be called after setting `#ready` or else it will not schedule sleep
		this.#resetSleepTimer();

		// Trigger any pending alarms
		await this._onAlarm();
	}

	#assertReady(allowStoppingState: boolean = false) {
		if (!this.#ready) throw new errors.InternalError("Actor not ready");
		if (!allowStoppingState && this.#stopCalled)
			throw new errors.InternalError("Actor is stopping");
	}

	/**
	 * Check if the actor is ready to handle requests.
	 */
	isReady(): boolean {
		return this.#ready;
	}

	// MARK: Stop
	/**
	 * For the engine:
	 * 1. Engine runner receives CommandStopActor
	 * 2. Engine runner calls _onStop and waits for it to finish
	 * 3. Engine runner publishes EventActorStateUpdate with ActorStateSTop
	 */
	async _onStop() {
		if (this.#stopCalled) {
			this.#rLog.warn({ msg: "already stopping actor" });
			return;
		}
		this.#stopCalled = true;

		this.#rLog.info({ msg: "actor stopping" });

		if (this.#sleepTimeout) {
			clearTimeout(this.#sleepTimeout);
			this.#sleepTimeout = undefined;
		}

		// Abort any listeners waiting for shutdown
		try {
			this.#abortController.abort();
		} catch {}

		// Call onStop lifecycle hook if defined
		if (this.#config.onStop) {
			try {
				this.#rLog.debug({ msg: "calling onStop" });
				const result = this.#config.onStop(this.actorContext);
				if (result instanceof Promise) {
					await deadline(result, this.#config.options.onStopTimeout);
				}
				this.#rLog.debug({ msg: "onStop completed" });
			} catch (error) {
				if (error instanceof DeadlineError) {
					this.#rLog.error({ msg: "onStop timed out" });
				} else {
					this.#rLog.error({
						msg: "error in onStop",
						error: stringifyError(error),
					});
				}
			}
		}

		const promises: Promise<unknown>[] = [];

		// Disconnect existing non-hibernatable connections
		for (const connection of this.#connections.values()) {
			if (!connection.isHibernatable) {
				this.#rLog.debug({
					msg: "disconnecting non-hibernatable connection on actor stop",
					connId: connection.id,
				});
				promises.push(connection.disconnect());
			}

			// TODO: Figure out how to abort HTTP requests on shutdown. This
			// might already be handled by the engine runner tunnel shutdown.
		}

		// Wait for any background tasks to finish, with timeout
		await this.#waitBackgroundPromises(
			this.#config.options.waitUntilTimeout,
		);

		// Clear timeouts
		if (this.#pendingSaveTimeout) clearTimeout(this.#pendingSaveTimeout);

		// Write state
		await this.saveState({ immediate: true, allowStoppingState: true });

		// Await all `close` event listeners with 1.5 second timeout
		const res = Promise.race([
			Promise.all(promises).then(() => false),
			new Promise<boolean>((res) =>
				globalThis.setTimeout(() => res(true), 1500),
			),
		]);

		if (await res) {
			this.#rLog.warn({
				msg: "timed out waiting for connections to close, shutting down anyway",
			});
		}

		// Wait for queues to finish
		if (this.#persistWriteQueue.runningDrainLoop)
			await this.#persistWriteQueue.runningDrainLoop;
		if (this.#alarmWriteQueue.runningDrainLoop)
			await this.#alarmWriteQueue.runningDrainLoop;
	}

	/** Abort signal that fires when the actor is stopping. */
	get abortSignal(): AbortSignal {
		return this.#abortController.signal;
	}

	// MARK: Sleep
	/**
	 * Reset timer from the last actor interaction that allows it to be put to sleep.
	 *
	 * This should be called any time a sleep-related event happens:
	 * - Connection opens (will clear timer)
	 * - Connection closes (will schedule timer if there are no open connections)
	 * - Alarm triggers (will reset timer)
	 *
	 * We don't need to call this on events like individual action calls, since there will always be a connection open for these.
	 **/
	#resetSleepTimer() {
		if (this.#config.options.noSleep || !this.#sleepingSupported) return;

		// Don't sleep if already stopping
		if (this.#stopCalled) return;

		const canSleep = this.#canSleep();

		this.#rLog.debug({
			msg: "resetting sleep timer",
			canSleep: CanSleep[canSleep],
			existingTimeout: !!this.#sleepTimeout,
			timeout: this.#config.options.sleepTimeout,
		});

		if (this.#sleepTimeout) {
			clearTimeout(this.#sleepTimeout);
			this.#sleepTimeout = undefined;
		}

		// Don't set a new timer if already sleeping
		if (this.#sleepCalled) return;

		if (canSleep === CanSleep.Yes) {
			this.#sleepTimeout = setTimeout(() => {
				this._startSleep();
			}, this.#config.options.sleepTimeout);
		}
	}

	/** If this actor can be put in a sleeping state. */
	#canSleep(): CanSleep {
		if (!this.#ready) return CanSleep.NotReady;

		// Do not sleep if Hono HTTP requests are in-flight
		if (this.#activeHonoHttpRequests > 0)
			return CanSleep.ActiveHonoHttpRequests;

		// TODO: When WS hibernation is ready, update this to only count non-hibernatable websockets
		// Do not sleep if there are raw websockets open
		if (this.#activeRawWebSockets.size > 0)
			return CanSleep.ActiveRawWebSockets;

		// Check for active conns. This will also cover active actions, since all actions have a connection.
		for (const conn of this.#connections.values()) {
			// TODO: Enable this when hibernation is implemented. We're waiting on support for Guard to not auto-wake the actor if it sleeps.
			// if (!conn.isHibernatable)
			// 	return false;

			// if (!conn.isHibernatable) return CanSleep.ActiveConns;
			return CanSleep.ActiveConns;
		}

		return CanSleep.Yes;
	}

	/**
	 * Puts an actor to sleep. This should just start the sleep sequence, most shutdown logic should be in _stop (which is called by the ActorDriver when sleeping).
	 *
	 * For the engine, this will:
	 * 1. Publish EventActorIntent with ActorIntentSleep (via driver.startSleep)
	 * 2. Engine runner will wait for CommandStopActor
	 * 3. Engine runner will call _onStop and wait for it to finish
	 * 4. Engine runner will publish EventActorStateUpdate with ActorStateSTop
	 **/
	_startSleep() {
		if (this.#stopCalled) {
			this.#rLog.debug({
				msg: "cannot call _startSleep if actor already stopping",
			});
			return;
		}

		// IMPORTANT: #sleepCalled should have no effect on the actor's
		// behavior aside from preventing calling _startSleep twice. Wait for
		// `_onStop` before putting in a stopping state.
		if (this.#sleepCalled) {
			this.#rLog.warn({
				msg: "cannot call _startSleep twice, actor already sleeping",
			});
			return;
		}
		this.#sleepCalled = true;

		// NOTE: Publishes ActorIntentSleep
		const sleep = this.#actorDriver.startSleep?.bind(
			this.#actorDriver,
			this.#actorId,
		);
		invariant(this.#sleepingSupported, "sleeping not supported");
		invariant(sleep, "no sleep on driver");

		this.#rLog.info({ msg: "actor sleeping" });

		// Schedule sleep to happen on the next tick. This allows for any action that calls _sleep to complete.
		setImmediate(() => {
			// The actor driver should call stop when ready to stop
			//
			// This will call _stop once Pegboard responds with the new status
			sleep();
		});
	}

	/**
	 * Called by router middleware when an HTTP request begins.
	 */
	__beginHonoHttpRequest() {
		this.#activeHonoHttpRequests++;
		this.#resetSleepTimer();
	}

	/**
	 * Called by router middleware when an HTTP request ends.
	 */
	__endHonoHttpRequest() {
		this.#activeHonoHttpRequests--;
		if (this.#activeHonoHttpRequests < 0) {
			this.#activeHonoHttpRequests = 0;
			this.#rLog.warn({
				msg: "active hono requests went below 0, this is a RivetKit bug",
				...EXTRA_ERROR_LOG,
			});
		}
		this.#resetSleepTimer();
	}

	// MARK: State
	/**
	 * Gets the current state.
	 *
	 * Changing properties of this value will automatically be persisted.
	 */
	get state(): S {
		this.#validateStateEnabled();
		return this.#persist.state;
	}

	/**
	 * Sets the current state.
	 *
	 * This property will automatically be persisted.
	 */
	set state(value: S) {
		this.#validateStateEnabled();
		this.#persist.state = value;
	}

	get stateEnabled() {
		return "createState" in this.#config || "state" in this.#config;
	}

	#validateStateEnabled() {
		if (!this.stateEnabled) {
			throw new errors.StateNotEnabled();
		}
	}

	get connStateEnabled() {
		return "createConnState" in this.#config || "connState" in this.#config;
	}

	get vars(): V {
		this.#validateVarsEnabled();
		invariant(this.#vars !== undefined, "vars not enabled");
		return this.#vars;
	}

	get #varsEnabled() {
		return "createVars" in this.#config || "vars" in this.#config;
	}

	#validateVarsEnabled() {
		if (!this.#varsEnabled) {
			throw new errors.VarsNotEnabled();
		}
	}

	/**
	 * Forces the state to get saved.
	 *
	 * This is helpful if running a long task that may fail later or when
	 * running a background job that updates the state.
	 *
	 * @param opts - Options for saving the state.
	 */
	async saveState(opts: SaveStateOptions) {
		this.#assertReady(opts.allowStoppingState);

		this.#rLog.debug({
			msg: "saveState called",
			persistChanged: this.#persistChanged,
			allowStoppingState: opts.allowStoppingState,
			immediate: opts.immediate,
		});

		if (this.#persistChanged) {
			if (opts.immediate) {
				// Save immediately
				await this.#savePersistInner();
			} else {
				// Create callback
				if (!this.#onPersistSavedPromise) {
					this.#onPersistSavedPromise = promiseWithResolvers();
				}

				// Save state throttled
				this.#savePersistThrottled();

				// Wait for save
				await this.#onPersistSavedPromise.promise;
			}
		}
	}

	/** Promise used to wait for a save to complete. This is required since you cannot await `#saveStateThrottled`. */
	#onPersistSavedPromise?: ReturnType<typeof promiseWithResolvers<void>>;

	/** Throttled save state method. Used to write to KV at a reasonable cadence. */
	#savePersistThrottled() {
		const now = Date.now();
		const timeSinceLastSave = now - this.#lastSaveTime;
		const saveInterval = this.#config.options.stateSaveInterval;

		// If we're within the throttle window and not already scheduled, schedule the next save.
		if (timeSinceLastSave < saveInterval) {
			if (this.#pendingSaveTimeout === undefined) {
				this.#pendingSaveTimeout = setTimeout(() => {
					this.#pendingSaveTimeout = undefined;
					this.#savePersistInner();
				}, saveInterval - timeSinceLastSave);
			}
		} else {
			// If we're outside the throttle window, save immediately
			this.#savePersistInner();
		}
	}

	/** Saves the state to KV. You probably want to use #saveStateThrottled instead except for a few edge cases. */
	async #savePersistInner() {
		try {
			this.#lastSaveTime = Date.now();

			const hasChanges =
				this.#persistChanged || this.#changedConnections.size > 0;

			if (hasChanges) {
				const finished = this.#persistWriteQueue.enqueue(async () => {
					this.#rLog.debug({
						msg: "saving persist",
						actorChanged: this.#persistChanged,
						connectionsChanged: this.#changedConnections.size,
					});

					await this.#writePersistedData();

					this.#rLog.debug({ msg: "persist saved" });
				});

				await finished;
			}

			this.#onPersistSavedPromise?.resolve();
		} catch (error) {
			this.#rLog.error({
				msg: "error saving persist",
				error: stringifyError(error),
			});
			this.#onPersistSavedPromise?.reject(error);
			throw error;
		}
	}

	async #writePersistedData() {
		const entries: [Uint8Array, Uint8Array][] = [];

		// Save actor state if changed
		if (this.#persistChanged) {
			this.#persistChanged = false;

			// Prepare actor state
			const bareData = this.#convertToBarePersisted(this.#persistRaw);

			// Key [1] for actor persist data
			entries.push([
				KEYS.PERSIST_DATA,
				ACTOR_VERSIONED.serializeWithEmbeddedVersion(bareData),
			]);
		}

		// Save changed connections
		if (this.#changedConnections.size > 0) {
			for (const connId of this.#changedConnections) {
				const conn = this.#connections.get(connId);
				if (conn) {
					const connData = cbor.encode(conn.persistRaw);
					entries.push([makeConnKey(connId), connData]);
					conn.markSaved();
				}
			}
			this.#changedConnections.clear();
		}

		// Write all entries in batch
		if (entries.length > 0) {
			await this.#actorDriver.kvBatchPut(this.#actorId, entries);
		}
	}

	/**
	 * Creates proxy for `#persist` that handles automatically flagging when state needs to be updated.
	 */
	#initPersistProxy(target: PersistedActor<S, CP, CS, I>) {
		// Set raw persist object
		this.#persistRaw = target;

		// TODO: Allow disabling in production
		// If this can't be proxied, return raw value
		if (target === null || typeof target !== "object") {
			let invalidPath = "";
			if (
				!isCborSerializable(
					target,
					(path) => {
						invalidPath = path;
					},
					"",
				)
			) {
				throw new errors.InvalidStateType({ path: invalidPath });
			}
			return target;
		}

		// Unsubscribe from old state
		if (this.#persist) {
			onChange.unsubscribe(this.#persist);
		}

		// Listen for changes to the object in order to automatically write state
		this.#persist = onChange(
			target,
			// biome-ignore lint/suspicious/noExplicitAny: Don't know types in proxy
			(
				path: string,
				value: any,
				_previousValue: any,
				_applyData: any,
			) => {
				const actorStatePath = isStatePath(path);
				const connStatePath = isConnStatePath(path);

				// Validate CBOR serializability for state changes
				if (actorStatePath || connStatePath) {
					let invalidPath = "";
					if (
						!isCborSerializable(
							value,
							(invalidPathPart) => {
								invalidPath = invalidPathPart;
							},
							"",
						)
					) {
						throw new errors.InvalidStateType({
							path: path + (invalidPath ? `.${invalidPath}` : ""),
						});
					}
				}

				this.#rLog.debug({
					msg: "onChange triggered, setting persistChanged=true",
					path,
				});
				this.#persistChanged = true;

				// Inform the inspector about state changes (only for state path)
				if (actorStatePath) {
					this.inspector.emitter.emit(
						"stateUpdated",
						this.#persist.state,
					);
				}

				// Call onStateChange if it exists
				//
				// Skip if we're already inside onStateChange to prevent infinite recursion
				if (
					actorStatePath &&
					this.#config.onStateChange &&
					this.#ready &&
					!this.#isInOnStateChange
				) {
					try {
						this.#isInOnStateChange = true;
						this.#config.onStateChange(
							this.actorContext,
							this.#persistRaw.state,
						);
					} catch (error) {
						this.#rLog.error({
							msg: "error in `_onStateChange`",
							error: stringifyError(error),
						});
					} finally {
						this.#isInOnStateChange = false;
					}
				}

				// State will be flushed at the end of the action
			},
			{ ignoreDetached: true },
		);
	}

	// MARK: Connections
	__getConnForId(id: string): Conn<S, CP, CS, V, I, DB> | undefined {
		return this.#connections.get(id);
	}

	/**
	 * Mark a connection as changed so it will be persisted on next save
	 */
	__markConnChanged(conn: Conn<S, CP, CS, V, I, DB>) {
		this.#changedConnections.add(conn.id);
		this.#rLog.debug({
			msg: "marked connection as changed",
			connId: conn.id,
			totalChanged: this.#changedConnections.size,
		});
	}

	/**
	 * Call when conn is disconnected.
	 *
	 * If a clean diconnect, will be removed immediately.
	 *
	 * If not a clean disconnect, will keep the connection alive for a given interval to wait for reconnect.
	 */
	__connDisconnected(
		conn: Conn<S, CP, CS, V, I, DB>,
		wasClean: boolean,
		requestId: string,
	) {
		// If socket ID is provided, check if it matches the current socket ID
		// If it doesn't match, this is a stale disconnect event from an old socket
		if (
			requestId &&
			conn.__socket &&
			requestId !== conn.__socket.requestId
		) {
			this.#rLog.debug({
				msg: "ignoring stale disconnect event",
				connId: conn.id,
				eventRequestId: requestId,
				currentRequestId: conn.__socket.requestId,
			});
			return;
		}

		if (wasClean) {
			// Disconnected cleanly, remove the conn

			this.#removeConn(conn);
		} else {
			// Disconnected uncleanly, allow reconnection

			if (!conn.__driverState) {
				this.rLog.warn("called conn disconnected without driver state");
			}

			// Update last seen so we know when to clean it up
			conn.__persist.lastSeen = Date.now();

			// Remove socket
			conn.__socket = undefined;

			// Update sleep
			this.#resetSleepTimer();
		}
	}

	/**
	 * Removes a connection and cleans up its resources.
	 */
	#removeConn(conn: Conn<S, CP, CS, V, I, DB>) {
		// Remove conn from KV
		const key = makeConnKey(conn.id);
		this.#actorDriver
			.kvBatchDelete(this.#actorId, [key])
			.then(() => {
				this.#rLog.debug({
					msg: "removed connection from KV",
					connId: conn.id,
				});
			})
			.catch((err) => {
				this.#rLog.error({
					msg: "kvBatchDelete failed for conn",
					err: stringifyError(err),
				});
			});

		// Remove from state and tracking
		this.#connections.delete(conn.id);
		this.#changedConnections.delete(conn.id);
		this.#rLog.debug({ msg: "removed conn", connId: conn.id });

		// Remove subscriptions
		for (const eventName of [...conn.subscriptions.values()]) {
			this.#removeSubscription(eventName, conn, true);
		}

		this.inspector.emitter.emit("connectionUpdated");
		if (this.#config.onDisconnect) {
			try {
				const result = this.#config.onDisconnect(
					this.actorContext,
					conn,
				);
				if (result instanceof Promise) {
					// Handle promise but don't await it to prevent blocking
					result.catch((error) => {
						this.#rLog.error({
							msg: "error in `onDisconnect`",
							error: stringifyError(error),
						});
					});
				}
			} catch (error) {
				this.#rLog.error({
					msg: "error in `onDisconnect`",
					error: stringifyError(error),
				});
			}
		}

		// Update sleep
		this.#resetSleepTimer();
	}

	/**
	 * Called to create a new connection or reconnect an existing one.
	 */
	async createConn(
		socket: ConnSocket,
		// biome-ignore lint/suspicious/noExplicitAny: TypeScript bug with ExtractActorConnParams<this>,
		params: any,
		request?: Request,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		this.#assertReady();

		// TODO: Remove this for ws hibernation v2 since we don't receive an open message for ws
		// Check for hibernatable websocket reconnection
		if (socket.requestIdBuf && socket.hibernatable) {
			this.rLog.debug({
				msg: "checking for hibernatable websocket connection",
				requestId: socket.requestId,
				existingConnectionsCount: this.#connections.size,
			});

			// Find existing connection with matching hibernatableRequestId
			const existingConn = Array.from(this.#connections.values()).find(
				(conn) =>
					conn.__persist.hibernatableRequestId &&
					arrayBuffersEqual(
						conn.__persist.hibernatableRequestId,
						socket.requestIdBuf!,
					),
			);

			if (existingConn) {
				this.rLog.debug({
					msg: "reconnecting hibernatable websocket connection",
					connectionId: existingConn.id,
					requestId: socket.requestId,
				});

				// If there's an existing driver state, clean it up without marking as clean disconnect
				if (existingConn.__driverState) {
					this.#rLog.warn({
						msg: "found existing driver state on hibernatable websocket",
						connectionId: existingConn.id,
						requestId: socket.requestId,
					});
					const driverKind = getConnDriverKindFromState(
						existingConn.__driverState,
					);
					const driver = CONN_DRIVERS[driverKind];
					if (driver.disconnect) {
						// Call driver disconnect to clean up directly. Don't use Conn.disconnect since that will remove the connection entirely.
						driver.disconnect(
							this,
							existingConn,
							(existingConn.__driverState as any)[driverKind],
							"Reconnecting hibernatable websocket with new driver state",
						);
					}
				}

				// Update with new driver state
				existingConn.__socket = socket;
				existingConn.__persist.lastSeen = Date.now();

				// Update sleep timer since connection is now active
				this.#resetSleepTimer();

				this.inspector.emitter.emit("connectionUpdated");

				// We don't need to send a new init message since this is a
				// hibernated request that has already been initialized

				return existingConn;
			} else {
				this.rLog.debug({
					msg: "no existing hibernatable connection found, creating new connection",
					requestId: socket.requestId,
				});
			}
		}

		// Prepare connection state
		let connState: CS | undefined;

		const onBeforeConnectOpts = {
			request,
		} satisfies OnConnectOptions;

		if (this.#config.onBeforeConnect) {
			await this.#config.onBeforeConnect(
				this.actorContext,
				onBeforeConnectOpts,
				params,
			);
		}

		if (this.connStateEnabled) {
			if ("createConnState" in this.#config) {
				const dataOrPromise = this.#config.createConnState(
					this.actorContext as unknown as ActorContext<
						undefined,
						undefined,
						undefined,
						undefined,
						undefined,
						undefined
					>,
					onBeforeConnectOpts,
					params,
				);
				if (dataOrPromise instanceof Promise) {
					connState = await deadline(
						dataOrPromise,
						this.#config.options.createConnStateTimeout,
					);
				} else {
					connState = dataOrPromise;
				}
			} else if ("connState" in this.#config) {
				connState = structuredClone(this.#config.connState);
			} else {
				throw new Error(
					"Could not create connection state from 'createConnState' or 'connState'",
				);
			}
		}

		// Create connection
		const persist: PersistedConn<CP, CS> = {
			connId: crypto.randomUUID(),
			params: params,
			state: connState as CS,
			lastSeen: Date.now(),
			subscriptions: [],
		};

		// Check if this connection is for a hibernatable websocket
		if (socket.requestIdBuf) {
			const isHibernatable =
				this.#persist.hibernatableConns.findIndex((conn) =>
					arrayBuffersEqual(
						conn.hibernatableRequestId,
						socket.requestIdBuf!,
					),
				) !== -1;

			if (isHibernatable) {
				persist.hibernatableRequestId = socket.requestIdBuf;
			}
		}

		const conn = new Conn<S, CP, CS, V, I, DB>(this, persist);
		conn.__socket = socket;
		this.#connections.set(conn.id, conn);

		// Update sleep
		//
		// Do this immediately after adding connection & before any async logic in order to avoid race conditions with sleep timeouts
		this.#resetSleepTimer();

		// Mark connection as changed for batch save
		this.#changedConnections.add(conn.id);

		this.saveState({ immediate: true });

		// Handle connection
		if (this.#config.onConnect) {
			try {
				const result = this.#config.onConnect(this.actorContext, conn);
				if (result instanceof Promise) {
					deadline(
						result,
						this.#config.options.onConnectTimeout,
					).catch((error) => {
						this.#rLog.error({
							msg: "error in `onConnect`, closing socket",
							error,
						});
						conn?.disconnect("`onConnect` failed");
					});
				}
			} catch (error) {
				this.#rLog.error({
					msg: "error in `onConnect`",
					error: stringifyError(error),
				});
				conn?.disconnect("`onConnect` failed");
			}
		}

		this.inspector.emitter.emit("connectionUpdated");

		// Send init message
		conn._sendMessage(
			new CachedSerializer<protocol.ToClient>(
				{
					body: {
						tag: "Init",
						val: {
							actorId: this.id,
							connectionId: conn.id,
						},
					},
				},
				TO_CLIENT_VERSIONED,
			),
		);

		return conn;
	}

	// MARK: Messages
	async processMessage(
		message: protocol.ToServer,
		conn: Conn<S, CP, CS, V, I, DB>,
	) {
		await processMessage(message, this, conn, {
			onExecuteAction: async (ctx, name, args) => {
				this.inspector.emitter.emit("eventFired", {
					type: "action",
					name,
					args,
					connId: conn.id,
				});
				return await this.executeAction(ctx, name, args);
			},
			onSubscribe: async (eventName, conn) => {
				this.inspector.emitter.emit("eventFired", {
					type: "subscribe",
					eventName,
					connId: conn.id,
				});
				this.#addSubscription(eventName, conn, false);
			},
			onUnsubscribe: async (eventName, conn) => {
				this.inspector.emitter.emit("eventFired", {
					type: "unsubscribe",
					eventName,
					connId: conn.id,
				});
				this.#removeSubscription(eventName, conn, false);
			},
		});
	}

	// MARK: Actions
	/**
	 * Execute an action call from a client.
	 *
	 * This method handles:
	 * 1. Validating the action name
	 * 2. Executing the action function
	 * 3. Processing the result through onBeforeActionResponse (if configured)
	 * 4. Handling timeouts and errors
	 * 5. Saving state changes
	 *
	 * @param ctx The action context
	 * @param actionName The name of the action being called
	 * @param args The arguments passed to the action
	 * @returns The result of the action call
	 * @throws {ActionNotFound} If the action doesn't exist
	 * @throws {ActionTimedOut} If the action times out
	 * @internal
	 */
	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		invariant(this.#ready, "executing action before ready");

		// Prevent calling private or reserved methods
		if (!(actionName in this.#config.actions)) {
			this.#rLog.warn({ msg: "action does not exist", actionName });
			throw new errors.ActionNotFound(actionName);
		}

		// Check if the method exists on this object
		const actionFunction = this.#config.actions[actionName];
		if (typeof actionFunction !== "function") {
			this.#rLog.warn({
				msg: "action is not a function",
				actionName: actionName,
				type: typeof actionFunction,
			});
			throw new errors.ActionNotFound(actionName);
		}

		// TODO: pass abortable to the action to decide when to abort
		// TODO: Manually call abortable for better error handling
		// Call the function on this object with those arguments
		try {
			// Log when we start executing the action
			this.#rLog.debug({
				msg: "executing action",
				actionName: actionName,
				args,
			});

			const outputOrPromise = actionFunction.call(
				undefined,
				ctx,
				...args,
			);
			let output: unknown;
			if (outputOrPromise instanceof Promise) {
				// Log that we're waiting for an async action
				this.#rLog.debug({
					msg: "awaiting async action",
					actionName: actionName,
				});

				output = await deadline(
					outputOrPromise,
					this.#config.options.actionTimeout,
				);

				// Log that async action completed
				this.#rLog.debug({
					msg: "async action completed",
					actionName: actionName,
				});
			} else {
				output = outputOrPromise;
			}

			// Process the output through onBeforeActionResponse if configured
			if (this.#config.onBeforeActionResponse) {
				try {
					const processedOutput = this.#config.onBeforeActionResponse(
						this.actorContext,
						actionName,
						args,
						output,
					);
					if (processedOutput instanceof Promise) {
						this.#rLog.debug({
							msg: "awaiting onBeforeActionResponse",
							actionName: actionName,
						});
						output = await processedOutput;
						this.#rLog.debug({
							msg: "onBeforeActionResponse completed",
							actionName: actionName,
						});
					} else {
						output = processedOutput;
					}
				} catch (error) {
					this.#rLog.error({
						msg: "error in `onBeforeActionResponse`",
						error: stringifyError(error),
					});
				}
			}

			// Log the output before returning
			this.#rLog.debug({
				msg: "action completed",
				actionName: actionName,
				outputType: typeof output,
				isPromise: output instanceof Promise,
			});

			// This output *might* reference a part of the state (using onChange), but
			// that's OK since this value always gets serialized and sent over the
			// network.
			return output;
		} catch (error) {
			if (error instanceof DeadlineError) {
				throw new errors.ActionTimedOut();
			}
			this.#rLog.error({
				msg: "action error",
				actionName: actionName,
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#savePersistThrottled();
		}
	}

	/**
	 * Returns a list of action methods available on this actor.
	 */
	get actions(): string[] {
		return Object.keys(this.#config.actions);
	}

	/**
	 * Handles raw HTTP requests to the actor.
	 */
	async handleFetch(
		request: Request,
		opts: Record<never, never>,
	): Promise<Response> {
		this.#assertReady();

		if (!this.#config.onFetch) {
			throw new errors.FetchHandlerNotDefined();
		}

		try {
			const response = await this.#config.onFetch(
				this.actorContext,
				request,
				opts,
			);
			if (!response) {
				throw new errors.InvalidFetchResponse();
			}
			return response;
		} catch (error) {
			this.#rLog.error({
				msg: "onFetch error",
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#savePersistThrottled();
		}
	}

	/**
	 * Handles raw WebSocket connections to the actor.
	 */
	async handleWebSocket(
		websocket: UniversalWebSocket,
		opts: { request: Request },
	): Promise<void> {
		this.#assertReady();

		if (!this.#config.onWebSocket) {
			throw new errors.InternalError("onWebSocket handler not defined");
		}

		try {
			// Set up state tracking to detect changes during WebSocket handling
			const stateBeforeHandler = this.#persistChanged;

			// Track active websocket until it fully closes
			this.#activeRawWebSockets.add(websocket);
			this.#resetSleepTimer();

			// Track hibernatable WebSockets
			let rivetRequestId: ArrayBuffer | undefined;
			let persistedHibernatableWebSocket:
				| PersistedHibernatableConn<CP, CS>
				| undefined;

			const onSocketOpened = (event: any) => {
				rivetRequestId = event?.rivetRequestId;

				// Find hibernatable WS
				if (rivetRequestId) {
					const rivetRequestIdLocal = rivetRequestId;
					persistedHibernatableWebSocket =
						this.#persist.hibernatableConns.find((conn) =>
							arrayBuffersEqual(
								conn.hibernatableRequestId,
								rivetRequestIdLocal,
							),
						);

					if (persistedHibernatableWebSocket) {
						persistedHibernatableWebSocket.lastSeenTimestamp =
							Date.now();
					}
				}

				this.#rLog.debug({
					msg: "actor instance onSocketOpened",
					rivetRequestId,
					isHibernatable: !!persistedHibernatableWebSocket,
					hibernationMsgIndex:
						persistedHibernatableWebSocket?.msgIndex,
				});
			};

			const onSocketMessage = (event: any) => {
				// Update state of hibernatable WS
				if (persistedHibernatableWebSocket) {
					persistedHibernatableWebSocket.lastSeenTimestamp =
						Date.now();
					persistedHibernatableWebSocket.msgIndex =
						event.rivetMessageIndex;
				}

				this.#rLog.debug({
					msg: "actor instance onSocketMessage",
					rivetRequestId,
					isHibernatable: !!persistedHibernatableWebSocket,
					hibernationMsgIndex:
						persistedHibernatableWebSocket?.msgIndex,
				});
			};

			const onSocketClosed = (_event: any) => {
				// Remove hibernatable WS
				if (rivetRequestId) {
					const rivetRequestIdLocal = rivetRequestId;
					const wsIndex = this.#persist.hibernatableConns.findIndex(
						(conn) =>
							arrayBuffersEqual(
								conn.hibernatableRequestId,
								rivetRequestIdLocal,
							),
					);

					const removed = this.#persist.hibernatableConns.splice(
						wsIndex,
						1,
					);
					if (removed.length > 0) {
						this.#rLog.debug({
							msg: "removed hibernatable websocket",
							rivetRequestId,
							hibernationMsgIndex:
								persistedHibernatableWebSocket?.msgIndex,
						});
					} else {
						this.#rLog.warn({
							msg: "could not find hibernatable websocket to remove",
							rivetRequestId,
							hibernationMsgIndex:
								persistedHibernatableWebSocket?.msgIndex,
						});
					}
				}

				this.#rLog.debug({
					msg: "actor instance onSocketMessage",
					rivetRequestId,
					isHibernatable: !!persistedHibernatableWebSocket,
					hibernatableWebSocketCount:
						this.#persist.hibernatableConns.length,
				});

				// Remove listener and socket from tracking
				try {
					websocket.removeEventListener("open", onSocketOpened);
					websocket.removeEventListener("message", onSocketMessage);
					websocket.removeEventListener("close", onSocketClosed);
					websocket.removeEventListener("error", onSocketClosed);
				} catch {}
				this.#activeRawWebSockets.delete(websocket);
				this.#resetSleepTimer();
			};

			try {
				websocket.addEventListener("open", onSocketOpened);
				websocket.addEventListener("message", onSocketMessage);
				websocket.addEventListener("close", onSocketClosed);
				websocket.addEventListener("error", onSocketClosed);
			} catch {}

			// Handle WebSocket
			await this.#config.onWebSocket(this.actorContext, websocket, opts);

			// If state changed during the handler, save it
			if (this.#persistChanged && !stateBeforeHandler) {
				await this.saveState({ immediate: true });
			}
		} catch (error) {
			this.#rLog.error({
				msg: "onWebSocket error",
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#savePersistThrottled();
		}
	}

	// MARK: Events
	#addSubscription(
		eventName: string,
		connection: Conn<S, CP, CS, V, I, DB>,
		fromPersist: boolean,
	) {
		if (connection.subscriptions.has(eventName)) {
			this.#rLog.debug({
				msg: "connection already has subscription",
				eventName,
			});
			return;
		}

		// Persist subscriptions & save immediately
		//
		// Don't update persistence if already restoring from persistence
		if (!fromPersist) {
			connection.__persist.subscriptions.push({ eventName: eventName });

			// Mark connection as changed
			this.#changedConnections.add(connection.id);

			this.saveState({ immediate: true });
		}

		// Update subscriptions
		connection.subscriptions.add(eventName);

		// Update subscription index
		let subscribers = this.#subscriptionIndex.get(eventName);
		if (!subscribers) {
			subscribers = new Set();
			this.#subscriptionIndex.set(eventName, subscribers);
		}
		subscribers.add(connection);
	}

	#removeSubscription(
		eventName: string,
		connection: Conn<S, CP, CS, V, I, DB>,
		fromRemoveConn: boolean,
	) {
		if (!connection.subscriptions.has(eventName)) {
			this.#rLog.warn({
				msg: "connection does not have subscription",
				eventName,
			});
			return;
		}

		// Persist subscriptions & save immediately
		//
		// Don't update the connection itself if the connection is already being removed
		if (!fromRemoveConn) {
			connection.subscriptions.delete(eventName);

			const subIdx = connection.__persist.subscriptions.findIndex(
				(s) => s.eventName === eventName,
			);
			if (subIdx !== -1) {
				connection.__persist.subscriptions.splice(subIdx, 1);
			} else {
				this.#rLog.warn({
					msg: "subscription does not exist with name",
					eventName,
				});
			}

			// Mark connection as changed
			this.#changedConnections.add(connection.id);

			this.saveState({ immediate: true });
		}

		// Update scriptions index
		const subscribers = this.#subscriptionIndex.get(eventName);
		if (subscribers) {
			subscribers.delete(connection);
			if (subscribers.size === 0) {
				this.#subscriptionIndex.delete(eventName);
			}
		}
	}

	/**
	 * Broadcasts an event to all connected clients.
	 * @param name - The name of the event.
	 * @param args - The arguments to send with the event.
	 */
	_broadcast<Args extends Array<unknown>>(name: string, ...args: Args) {
		this.#assertReady();

		this.inspector.emitter.emit("eventFired", {
			type: "broadcast",
			eventName: name,
			args,
		});

		// Send to all connected clients
		const subscriptions = this.#subscriptionIndex.get(name);
		if (!subscriptions) return;

		const toClientSerializer = new CachedSerializer<protocol.ToClient>(
			{
				body: {
					tag: "Event",
					val: {
						name,
						args: bufferToArrayBuffer(cbor.encode(args)),
					},
				},
			},
			TO_CLIENT_VERSIONED,
		);

		// Send message to clients
		for (const connection of subscriptions) {
			connection._sendMessage(toClientSerializer);
		}
	}

	// MARK: Alarms
	async #scheduleEventInner(newEvent: PersistedScheduleEvent) {
		this.actorContext.log.info({ msg: "scheduling event", ...newEvent });

		// Insert event in to index
		const insertIndex = this.#persist.scheduledEvents.findIndex(
			(x) => x.timestamp > newEvent.timestamp,
		);
		if (insertIndex === -1) {
			this.#persist.scheduledEvents.push(newEvent);
		} else {
			this.#persist.scheduledEvents.splice(insertIndex, 0, newEvent);
		}

		// Update alarm if:
		// - this is the newest event (i.e. at beginning of array) or
		// - this is the only event (i.e. the only event in the array)
		if (insertIndex === 0 || this.#persist.scheduledEvents.length === 1) {
			this.actorContext.log.info({
				msg: "setting alarm",
				timestamp: newEvent.timestamp,
				eventCount: this.#persist.scheduledEvents.length,
			});
			await this.#queueSetAlarm(newEvent.timestamp);
		}
	}

	async scheduleEvent(
		timestamp: number,
		action: string,
		args: unknown[],
	): Promise<void> {
		return this.#scheduleEventInner({
			eventId: crypto.randomUUID(),
			timestamp,
			action,
			args: bufferToArrayBuffer(cbor.encode(args)),
		});
	}

	/**
	 * Triggers any pending alarms.
	 *
	 * This method is idempotent. It's called automatically when the actor wakes
	 * in order to trigger any pending alarms.
	 */
	async _onAlarm() {
		const now = Date.now();
		this.actorContext.log.debug({
			msg: "alarm triggered",
			now,
			events: this.#persist.scheduledEvents.length,
		});

		// Update sleep
		//
		// Do this before any async logic
		this.#resetSleepTimer();

		// Remove events from schedule that we're about to run
		const runIndex = this.#persist.scheduledEvents.findIndex(
			(x) => x.timestamp <= now,
		);
		if (runIndex === -1) {
			// This method is idempotent, so this will happen in scenarios like `start` and
			// no events are pending.
			this.#rLog.debug({ msg: "no events are due yet" });
			if (this.#persist.scheduledEvents.length > 0) {
				const nextTs = this.#persist.scheduledEvents[0].timestamp;
				this.actorContext.log.debug({
					msg: "alarm fired early, rescheduling for next event",
					now,
					nextTs,
					delta: nextTs - now,
				});
				await this.#queueSetAlarm(nextTs);
			}
			this.actorContext.log.debug({ msg: "no events to run", now });
			return;
		}
		const scheduleEvents = this.#persist.scheduledEvents.splice(
			0,
			runIndex + 1,
		);
		this.actorContext.log.debug({
			msg: "running events",
			count: scheduleEvents.length,
		});

		// Set alarm for next event
		if (this.#persist.scheduledEvents.length > 0) {
			const nextTs = this.#persist.scheduledEvents[0].timestamp;
			this.actorContext.log.info({
				msg: "setting next alarm",
				nextTs,
				remainingEvents: this.#persist.scheduledEvents.length,
			});
			await this.#queueSetAlarm(nextTs);
		}

		// Iterate by event key in order to ensure we call the events in order
		for (const event of scheduleEvents) {
			try {
				this.actorContext.log.info({
					msg: "running action for event",
					event: event.eventId,
					timestamp: event.timestamp,
					action: event.action,
				});

				// Look up function
				const fn: unknown = this.#config.actions[event.action];

				if (!fn)
					throw new Error(`Missing action for alarm ${event.action}`);
				if (typeof fn !== "function")
					throw new Error(
						`Alarm function lookup for ${event.action} returned ${typeof fn}`,
					);

				// Call function
				try {
					const args = event.args
						? cbor.decode(new Uint8Array(event.args))
						: [];
					await fn.call(undefined, this.actorContext, ...args);
				} catch (error) {
					this.actorContext.log.error({
						msg: "error while running event",
						error: stringifyError(error),
						event: event.eventId,
						timestamp: event.timestamp,
						action: event.action,
					});
				}
			} catch (error) {
				this.actorContext.log.error({
					msg: "internal error while running event",
					error: stringifyError(error),
					...event,
				});
			}
		}
	}

	async #queueSetAlarm(timestamp: number): Promise<void> {
		await this.#alarmWriteQueue.enqueue(async () => {
			await this.#actorDriver.setAlarm(this, timestamp);
		});
	}

	// MARK: Background Promises
	/** Wait for background waitUntil promises with a timeout. */
	async #waitBackgroundPromises(timeoutMs: number) {
		const pending = this.#backgroundPromises;
		if (pending.length === 0) {
			this.#rLog.debug({ msg: "no background promises" });
			return;
		}

		// Race promises with timeout to determine if pending promises settled fast enough
		const timedOut = await Promise.race([
			Promise.allSettled(pending).then(() => false),
			new Promise<true>((resolve) =>
				setTimeout(() => resolve(true), timeoutMs),
			),
		]);

		if (timedOut) {
			this.#rLog.error({
				msg: "timed out waiting for background tasks, background promises may have leaked",
				count: pending.length,
				timeoutMs,
			});
		} else {
			this.#rLog.debug({ msg: "background promises finished" });
		}
	}

	/**
	 * Prevents the actor from sleeping until promise is complete.
	 *
	 * This allows the actor runtime to ensure that a promise completes while
	 * returning from an action request early.
	 *
	 * @param promise - The promise to run in the background.
	 */
	_waitUntil(promise: Promise<void>) {
		this.#assertReady();

		// TODO: Should we force save the state?
		// Add logging to promise and make it non-failable
		const nonfailablePromise = promise
			.then(() => {
				this.#rLog.debug({ msg: "wait until promise complete" });
			})
			.catch((error) => {
				this.#rLog.error({
					msg: "wait until promise failed",
					error: stringifyError(error),
				});
			});
		this.#backgroundPromises.push(nonfailablePromise);
	}

	// MARK: BARE Conversion Helpers
	#convertToBarePersisted(
		persist: PersistedActor<S, CP, CS, I>,
	): persistSchema.Actor {
		// Convert hibernatable connections from the in-memory connections map
		// Convert hibernatableConns from the persisted structure
		const hibernatableConns: persistSchema.HibernatableConn[] =
			persist.hibernatableConns.map((conn) => ({
				id: conn.id,
				parameters: bufferToArrayBuffer(
					cbor.encode(conn.parameters || {}),
				),
				state: bufferToArrayBuffer(cbor.encode(conn.state || {})),
				subscriptions: conn.subscriptions.map((sub) => ({
					eventName: sub.eventName,
				})),
				hibernatableRequestId: conn.hibernatableRequestId,
				lastSeenTimestamp: BigInt(conn.lastSeenTimestamp),
				msgIndex: BigInt(conn.msgIndex),
			}));

		return {
			input:
				persist.input !== undefined
					? bufferToArrayBuffer(cbor.encode(persist.input))
					: null,
			hasInitialized: persist.hasInitialized,
			state: bufferToArrayBuffer(cbor.encode(persist.state)),
			hibernatableConns,
			scheduledEvents: persist.scheduledEvents.map((event) => ({
				eventId: event.eventId,
				timestamp: BigInt(event.timestamp),
				action: event.action,
				args: event.args ?? null,
			})),
		};
	}

	#convertFromBarePersisted(
		bareData: persistSchema.Actor,
	): PersistedActor<S, CP, CS, I> {
		// Convert hibernatableConns from the BARE schema format
		const hibernatableConns: PersistedHibernatableConn<CP, CS>[] =
			bareData.hibernatableConns.map((conn) => ({
				id: conn.id,
				parameters: cbor.decode(new Uint8Array(conn.parameters)) as CP,
				state: cbor.decode(new Uint8Array(conn.state)) as CS,
				subscriptions: conn.subscriptions.map((sub) => ({
					eventName: sub.eventName,
				})),
				hibernatableRequestId: conn.hibernatableRequestId,
				lastSeenTimestamp: Number(conn.lastSeenTimestamp),
				msgIndex: Number(conn.msgIndex),
			}));

		return {
			input: bareData.input
				? cbor.decode(new Uint8Array(bareData.input))
				: undefined,
			hasInitialized: bareData.hasInitialized,
			state: cbor.decode(new Uint8Array(bareData.state)),
			hibernatableConns,
			scheduledEvents: bareData.scheduledEvents.map((event) => ({
				eventId: event.eventId,
				timestamp: Number(event.timestamp),
				action: event.action,
				args: event.args ?? undefined,
			})),
		};
	}
}
