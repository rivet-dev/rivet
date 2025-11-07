import * as cbor from "cbor-x";
import invariant from "invariant";
import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import { getBaseLogger, getIncludeTarget, type Logger } from "@/common/log";
import { stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { ActorInspector } from "@/inspector/actor";
import type { Registry } from "@/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import type * as protocol from "@/schemas/client-protocol/mod";
import { TO_CLIENT_VERSIONED } from "@/schemas/client-protocol/versioned";
import { EXTRA_ERROR_LOG, idToStr } from "@/utils";
import type { ActorConfig, InitContext } from "../config";
import type { ConnDriver } from "../conn/driver";
import { createHttpSocket } from "../conn/drivers/http";
import { CONN_PERSIST_SYMBOL, type Conn, type ConnId } from "../conn/mod";
import { ActionContext } from "../contexts/action";
import { ActorContext } from "../contexts/actor";
import type { AnyDatabaseProvider, InferDatabaseClient } from "../database";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { serializeActorKey } from "../keys";
import { processMessage } from "../protocol/old";
import { CachedSerializer } from "../protocol/serde";
import { Schedule } from "../schedule";
import { DeadlineError, deadline, generateSecureToken } from "../utils";
import { ConnectionManager } from "./connection-manager";
import { EventManager } from "./event-manager";
import { KEYS } from "./kv";
import type { PersistedActor, PersistedConn } from "./persisted";
import { ScheduleManager } from "./schedule-manager";
import { type SaveStateOptions, StateManager } from "./state-manager";

export type { SaveStateOptions };

export const ACTOR_INSTANCE_PERSIST_SYMBOL = Symbol("persist");

enum CanSleep {
	Yes,
	NotReady,
	ActiveConns,
	ActiveHonoHttpRequests,
}

/** Actor type alias with all `any` types. Used for `extends` in classes referencing this actor. */
export type AnyActorInstance = ActorInstance<any, any, any, any, any, any>;

export type ExtractActorState<A extends AnyActorInstance> =
	A extends ActorInstance<infer State, any, any, any, any, any>
		? State
		: never;

export type ExtractActorConnParams<A extends AnyActorInstance> =
	A extends ActorInstance<any, infer ConnParams, any, any, any, any>
		? ConnParams
		: never;

export type ExtractActorConnState<A extends AnyActorInstance> =
	A extends ActorInstance<any, any, infer ConnState, any, any, any>
		? ConnState
		: never;

// MARK: - Main ActorInstance Class
export class ActorInstance<S, CP, CS, V, I, DB extends AnyDatabaseProvider> {
	// MARK: - Core Properties
	actorContext: ActorContext<S, CP, CS, V, I, DB>;
	#config: ActorConfig<S, CP, CS, V, I, DB>;
	#actorDriver!: ActorDriver;
	#inlineClient!: Client<Registry<any>>;
	#actorId!: string;
	#name!: string;
	#key!: ActorKey;
	#region!: string;

	// MARK: - Managers
	#connectionManager!: ConnectionManager<S, CP, CS, V, I, DB>;
	#stateManager!: StateManager<S, CP, CS, I>;
	#eventManager!: EventManager<S, CP, CS, V, I, DB>;
	#scheduleManager!: ScheduleManager<S, CP, CS, V, I, DB>;

	// MARK: - Logging
	#log!: Logger;
	#rLog!: Logger;

	// MARK: - Lifecycle State
	#ready = false;
	#sleepCalled = false;
	#stopCalled = false;
	#sleepTimeout?: NodeJS.Timeout;
	#abortController = new AbortController();

	// MARK: - Variables & Database
	#vars?: V;
	#db!: InferDatabaseClient<DB>;

	// MARK: - Background Tasks
	#backgroundPromises: Promise<void>[] = [];

	// MARK: - HTTP/WebSocket Tracking
	#activeHonoHttpRequests = 0;

	// MARK: - Deprecated (kept for compatibility)
	#schedule!: Schedule;

	// MARK: - Inspector
	#inspectorToken?: string;
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
				if (!this.stateEnabled) {
					throw new errors.StateNotEnabled();
				}
				return this.#stateManager.persistRaw.state as Record<
					string,
					any
				> as unknown;
			},
			getRpcs: async () => {
				return Object.keys(this.#config.actions);
			},
			getConnections: async () => {
				return Array.from(
					this.#connectionManager.connections.entries(),
				).map(([id, conn]) => ({
					id,
					params: conn.params as any,
					state: conn.stateEnabled ? conn.state : undefined,
					subscriptions: conn.subscriptions.size,
					lastSeen: conn.lastSeen,
					stateEnabled: conn.stateEnabled,
					isHibernatable: conn.isHibernatable,
					hibernatableRequestId: conn[CONN_PERSIST_SYMBOL]
						.hibernatableRequestId
						? idToStr(
								conn[CONN_PERSIST_SYMBOL].hibernatableRequestId,
							)
						: undefined,
				}));
			},
			setState: async (state: unknown) => {
				if (!this.stateEnabled) {
					throw new errors.StateNotEnabled();
				}
				this.#stateManager.state = { ...(state as S) };
				await this.#stateManager.saveState({ immediate: true });
			},
			executeAction: async (name, params) => {
				const conn = await this.createConn(
					createHttpSocket(),
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
					this.connDisconnected(conn, true);
				}
			},
		};
	});

	// MARK: - Constructor
	constructor(config: ActorConfig<S, CP, CS, V, I, DB>) {
		this.#config = config;
		this.actorContext = new ActorContext(this);
	}

	// MARK: - Public Getters
	get log(): Logger {
		invariant(this.#log, "log not configured");
		return this.#log;
	}

	get rLog(): Logger {
		invariant(this.#rLog, "log not configured");
		return this.#rLog;
	}

	get isStopping(): boolean {
		return this.#stopCalled;
	}

	get id(): string {
		return this.#actorId;
	}

	get name(): string {
		return this.#name;
	}

	get key(): ActorKey {
		return this.#key;
	}

	get region(): string {
		return this.#region;
	}

	get inlineClient(): Client<Registry<any>> {
		return this.#inlineClient;
	}

	get inspector(): ActorInspector {
		return this.#inspector;
	}

	get inspectorToken(): string | undefined {
		return this.#inspectorToken;
	}

	get conns(): Map<ConnId, Conn<S, CP, CS, V, I, DB>> {
		return this.#connectionManager.connections;
	}

	get schedule(): Schedule {
		return this.#schedule;
	}

	get abortSignal(): AbortSignal {
		return this.#abortController.signal;
	}

	get actions(): string[] {
		return Object.keys(this.#config.actions);
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB> {
		return this.#config;
	}

	// MARK: - State Access
	get [ACTOR_INSTANCE_PERSIST_SYMBOL](): PersistedActor<S, CP, CS, I> {
		return this.#stateManager.persist;
	}

	get state(): S {
		return this.#stateManager.state;
	}

	set state(value: S) {
		this.#stateManager.state = value;
	}

	get stateEnabled(): boolean {
		return this.#stateManager.stateEnabled;
	}

	get connStateEnabled(): boolean {
		return "createConnState" in this.#config || "connState" in this.#config;
	}

	// MARK: - Variables & Database
	get vars(): V {
		this.#validateVarsEnabled();
		invariant(this.#vars !== undefined, "vars not enabled");
		return this.#vars;
	}

	get db(): InferDatabaseClient<DB> {
		if (!this.#db) {
			throw new errors.DatabaseNotEnabled();
		}
		return this.#db;
	}

	// MARK: - Initialization
	async start(
		actorDriver: ActorDriver,
		inlineClient: Client<Registry<any>>,
		actorId: string,
		name: string,
		key: ActorKey,
		region: string,
	) {
		// Initialize properties
		this.#actorDriver = actorDriver;
		this.#inlineClient = inlineClient;
		this.#actorId = actorId;
		this.#name = name;
		this.#key = key;
		this.#region = region;

		// Initialize logging
		this.#initializeLogging();

		// Initialize managers
		this.#connectionManager = new ConnectionManager(this);
		this.#stateManager = new StateManager(this, actorDriver, this.#config);
		this.#eventManager = new EventManager(this);
		this.#scheduleManager = new ScheduleManager(
			this,
			actorDriver,
			this.#config,
		);

		// Legacy schedule object (for compatibility)
		this.#schedule = new Schedule(this);

		// Read and initialize state
		await this.#initializeState();

		// Generate or load inspector token
		await this.#initializeInspectorToken();

		// Initialize variables
		if (this.#varsEnabled) {
			await this.#initializeVars();
		}

		// Call onStart lifecycle
		await this.#callOnStart();

		// Setup database
		await this.#setupDatabase();

		// Initialize alarms
		await this.#scheduleManager.initializeAlarms();

		// Mark as ready
		this.#ready = true;
		this.#rLog.info({ msg: "actor ready" });

		// Start sleep timer
		this.#resetSleepTimer();

		// Trigger any pending alarms
		await this.onAlarm();
	}

	// MARK: - Ready Check
	isReady(): boolean {
		return this.#ready;
	}

	#assertReady(allowStoppingState: boolean = false) {
		if (!this.#ready) throw new errors.InternalError("Actor not ready");
		if (!allowStoppingState && this.#stopCalled)
			throw new errors.InternalError("Actor is stopping");
	}

	// MARK: - Stop
	async onStop() {
		if (this.#stopCalled) {
			this.#rLog.warn({ msg: "already stopping actor" });
			return;
		}
		this.#stopCalled = true;
		this.#rLog.info({ msg: "actor stopping" });

		// Clear sleep timeout
		if (this.#sleepTimeout) {
			clearTimeout(this.#sleepTimeout);
			this.#sleepTimeout = undefined;
		}

		// Abort listeners
		try {
			this.#abortController.abort();
		} catch {}

		// Call onStop lifecycle
		await this.#callOnStop();

		// Disconnect non-hibernatable connections
		await this.#disconnectConnections();

		// Wait for background tasks
		await this.#waitBackgroundPromises(
			this.#config.options.waitUntilTimeout,
		);

		// Clear timeouts and save state
		this.#stateManager.clearPendingSaveTimeout();
		await this.saveState({ immediate: true, allowStoppingState: true });

		// Wait for write queues
		await this.#stateManager.waitForPendingWrites();
		await this.#scheduleManager.waitForPendingAlarmWrites();
	}

	// MARK: - Sleep
	startSleep() {
		if (this.#stopCalled) {
			this.#rLog.debug({
				msg: "cannot call startSleep if actor already stopping",
			});
			return;
		}

		if (this.#sleepCalled) {
			this.#rLog.warn({
				msg: "cannot call startSleep twice, actor already sleeping",
			});
			return;
		}
		this.#sleepCalled = true;

		const sleep = this.#actorDriver.startSleep?.bind(
			this.#actorDriver,
			this.#actorId,
		);
		invariant(this.#sleepingSupported, "sleeping not supported");
		invariant(sleep, "no sleep on driver");

		this.#rLog.info({ msg: "actor sleeping" });

		setImmediate(() => {
			sleep();
		});
	}

	// MARK: - HTTP Request Tracking
	beginHonoHttpRequest() {
		this.#activeHonoHttpRequests++;
		this.#resetSleepTimer();
	}

	endHonoHttpRequest() {
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

	// MARK: - State Management
	async saveState(opts: SaveStateOptions) {
		this.#assertReady(opts.allowStoppingState);

		// Save state through StateManager
		await this.#stateManager.saveState(opts);

		// Save connection changes
		if (this.#connectionManager.changedConnections.size > 0) {
			const entries = this.#connectionManager.getChangedConnectionsData();
			if (entries.length > 0) {
				await this.#actorDriver.kvBatchPut(this.#actorId, entries);
			}
			this.#connectionManager.clearChangedConnections();
		}
	}

	// MARK: - Connection Management
	getConnForId(id: string): Conn<S, CP, CS, V, I, DB> | undefined {
		return this.#connectionManager.getConnForId(id);
	}

	markConnChanged(conn: Conn<S, CP, CS, V, I, DB>) {
		this.#connectionManager.markConnChanged(conn);
	}

	connDisconnected(conn: Conn<S, CP, CS, V, I, DB>, wasClean: boolean) {
		this.#connectionManager.connDisconnected(
			conn,
			wasClean,
			this.#actorDriver,
			this.#eventManager,
		);
		this.#resetSleepTimer();
	}

	async createConn(
		driver: ConnDriver,
		params: any,
		request?: Request,
	): Promise<Conn<S, CP, CS, V, I, DB>> {
		this.#assertReady();

		const conn = await this.#connectionManager.createConn(
			driver,
			params,
			request,
		);

		// Reset sleep timer after connection
		this.#resetSleepTimer();

		// Save state immediately
		await this.saveState({ immediate: true });

		// Send init message
		conn.sendMessage(
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

	// MARK: - Message Processing
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
				this.#eventManager.addSubscription(eventName, conn, false);
			},
			onUnsubscribe: async (eventName, conn) => {
				this.inspector.emitter.emit("eventFired", {
					type: "unsubscribe",
					eventName,
					connId: conn.id,
				});
				this.#eventManager.removeSubscription(eventName, conn, false);
			},
		});
	}

	// MARK: - Action Execution
	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		invariant(this.#ready, "executing action before ready");

		if (!(actionName in this.#config.actions)) {
			this.#rLog.warn({ msg: "action does not exist", actionName });
			throw new errors.ActionNotFound(actionName);
		}

		const actionFunction = this.#config.actions[actionName];
		if (typeof actionFunction !== "function") {
			this.#rLog.warn({
				msg: "action is not a function",
				actionName,
				type: typeof actionFunction,
			});
			throw new errors.ActionNotFound(actionName);
		}

		try {
			this.#rLog.debug({
				msg: "executing action",
				actionName,
				args,
			});

			const outputOrPromise = actionFunction.call(
				undefined,
				ctx,
				...args,
			);

			let output: unknown;
			if (outputOrPromise instanceof Promise) {
				output = await deadline(
					outputOrPromise,
					this.#config.options.actionTimeout,
				);
			} else {
				output = outputOrPromise;
			}

			// Process through onBeforeActionResponse if configured
			if (this.#config.onBeforeActionResponse) {
				try {
					const processedOutput = this.#config.onBeforeActionResponse(
						this.actorContext,
						actionName,
						args,
						output,
					);
					if (processedOutput instanceof Promise) {
						output = await processedOutput;
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

			return output;
		} catch (error) {
			if (error instanceof DeadlineError) {
				throw new errors.ActionTimedOut();
			}
			this.#rLog.error({
				msg: "action error",
				actionName,
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#stateManager.savePersistThrottled();
		}
	}

	// MARK: - HTTP/WebSocket Handlers
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
			this.#stateManager.savePersistThrottled();
		}
	}

	async handleWebSocket(
		websocket: UniversalWebSocket,
		opts: { request: Request },
	): Promise<void> {
		this.#assertReady();

		if (!this.#config.onWebSocket) {
			throw new errors.InternalError("onWebSocket handler not defined");
		}

		try {
			const stateBeforeHandler = this.#stateManager.persistChanged;

			// Reset sleep timer when handling WebSocket
			this.#resetSleepTimer();

			// Handle WebSocket
			await this.#config.onWebSocket(this.actorContext, websocket, opts);

			// Save state if changed
			if (this.#stateManager.persistChanged && !stateBeforeHandler) {
				await this.saveState({ immediate: true });
			}
		} catch (error) {
			this.#rLog.error({
				msg: "onWebSocket error",
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.#stateManager.savePersistThrottled();
		}
	}

	// MARK: - Event Broadcasting
	broadcast<Args extends Array<unknown>>(name: string, ...args: Args) {
		this.#assertReady();
		this.#eventManager.broadcast(name, ...args);
	}

	// MARK: - Scheduling
	async scheduleEvent(
		timestamp: number,
		action: string,
		args: unknown[],
	): Promise<void> {
		await this.#scheduleManager.scheduleEvent(timestamp, action, args);
	}

	async onAlarm() {
		this.#resetSleepTimer();
		await this.#scheduleManager.onAlarm();
	}

	// MARK: - Background Tasks
	waitUntil(promise: Promise<void>) {
		this.#assertReady();

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

	// MARK: - Private Helper Methods
	#initializeLogging() {
		const logParams = {
			actor: this.#name,
			key: serializeActorKey(this.#key),
			actorId: this.#actorId,
		};

		const extraLogParams = this.#actorDriver.getExtraActorLogParams?.();
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
	}

	async #initializeState() {
		// Read initial state from KV
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
		const persistData =
			this.#stateManager.convertFromBarePersisted(bareData);

		if (persistData.hasInitialized) {
			// Restore existing actor
			await this.#restoreExistingActor(persistData);
		} else {
			// Create new actor
			await this.#createNewActor(persistData);
		}

		// Pass persist reference to schedule manager
		this.#scheduleManager.setPersist(this.#stateManager.persist);
	}

	async #restoreExistingActor(persistData: PersistedActor<S, CP, CS, I>) {
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

		// Initialize state
		this.#stateManager.initPersistProxy(persistData);

		// Restore connections
		this.#connectionManager.restoreConnections(
			connections,
			this.#eventManager,
		);
	}

	async #createNewActor(persistData: PersistedActor<S, CP, CS, I>) {
		this.#rLog.info({ msg: "actor creating" });

		// Initialize state
		await this.#stateManager.initializeState(persistData);

		// Call onCreate lifecycle
		if (this.#config.onCreate) {
			await this.#config.onCreate(this.actorContext, persistData.input!);
		}
	}

	async #initializeInspectorToken() {
		// Try to load existing token
		const [tokenBuffer] = await this.#actorDriver.kvBatchGet(
			this.#actorId,
			[KEYS.INSPECTOR_TOKEN],
		);

		if (tokenBuffer !== null) {
			// Token exists, decode it
			const decoder = new TextDecoder();
			this.#inspectorToken = decoder.decode(tokenBuffer);
			this.#rLog.debug({ msg: "loaded existing inspector token" });
		} else {
			// Generate new token
			this.#inspectorToken = generateSecureToken();
			const tokenBytes = new TextEncoder().encode(this.#inspectorToken);
			await this.#actorDriver.kvBatchPut(this.#actorId, [
				[KEYS.INSPECTOR_TOKEN, tokenBytes],
			]);
			this.#rLog.debug({ msg: "generated new inspector token" });
		}
	}

	async #initializeVars() {
		let vars: V | undefined;
		if ("createVars" in this.#config) {
			const dataOrPromise = this.#config.createVars(
				this.actorContext as unknown as InitContext,
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
				"Could not create variables from 'createVars' or 'vars'",
			);
		}
		this.#vars = vars;
	}

	async #callOnStart() {
		this.#rLog.info({ msg: "actor starting" });
		if (this.#config.onWake) {
			const result = this.#config.onWake(this.actorContext);
			if (result instanceof Promise) {
				await result;
			}
		}
	}

	async #callOnStop() {
		if (this.#config.onSleep) {
			try {
				this.#rLog.debug({ msg: "calling onStop" });
				const result = this.#config.onSleep(this.actorContext);
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
	}

	async #setupDatabase() {
		if ("db" in this.#config && this.#config.db) {
			const client = await this.#config.db.createClient({
				getDatabase: () => this.#actorDriver.getDatabase(this.#actorId),
			});
			this.#rLog.info({ msg: "database migration starting" });
			await this.#config.db.onMigrate?.(client);
			this.#rLog.info({ msg: "database migration complete" });
			this.#db = client;
		}
	}

	async #disconnectConnections() {
		const promises: Promise<unknown>[] = [];
		for (const connection of this.#connectionManager.connections.values()) {
			if (!connection.isHibernatable) {
				this.#rLog.debug({
					msg: "disconnecting non-hibernatable connection on actor stop",
					connId: connection.id,
				});
				promises.push(connection.disconnect());
			}
		}

		// Wait with timeout
		const res = await Promise.race([
			Promise.all(promises).then(() => false),
			new Promise<boolean>((res) =>
				globalThis.setTimeout(() => res(true), 1500),
			),
		]);

		if (res) {
			this.#rLog.warn({
				msg: "timed out waiting for connections to close, shutting down anyway",
			});
		}
	}

	async #waitBackgroundPromises(timeoutMs: number) {
		const pending = this.#backgroundPromises;
		if (pending.length === 0) {
			this.#rLog.debug({ msg: "no background promises" });
			return;
		}

		const timedOut = await Promise.race([
			Promise.allSettled(pending).then(() => false),
			new Promise<true>((resolve) =>
				setTimeout(() => resolve(true), timeoutMs),
			),
		]);

		if (timedOut) {
			this.#rLog.error({
				msg: "timed out waiting for background tasks",
				count: pending.length,
				timeoutMs,
			});
		} else {
			this.#rLog.debug({ msg: "background promises finished" });
		}
	}

	#resetSleepTimer() {
		if (this.#config.options.noSleep || !this.#sleepingSupported) return;
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

		if (this.#sleepCalled) return;

		if (canSleep === CanSleep.Yes) {
			this.#sleepTimeout = setTimeout(() => {
				this.startSleep();
			}, this.#config.options.sleepTimeout);
		}
	}

	#canSleep(): CanSleep {
		if (!this.#ready) return CanSleep.NotReady;
		if (this.#activeHonoHttpRequests > 0)
			return CanSleep.ActiveHonoHttpRequests;

		for (const _conn of this.#connectionManager.connections.values()) {
			return CanSleep.ActiveConns;
		}

		return CanSleep.Yes;
	}

	get #sleepingSupported(): boolean {
		return this.#actorDriver.startSleep !== undefined;
	}

	get #varsEnabled(): boolean {
		return "createVars" in this.#config || "vars" in this.#config;
	}

	#validateVarsEnabled() {
		if (!this.#varsEnabled) {
			throw new errors.VarsNotEnabled();
		}
	}
}
