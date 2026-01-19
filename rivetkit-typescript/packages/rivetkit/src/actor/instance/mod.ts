import invariant from "invariant";
import type { ActorKey } from "@/actor/mod";
import type { Client } from "@/client/client";
import { getBaseLogger, getIncludeTarget, type Logger } from "@/common/log";
import { stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { ActorInspector } from "@/inspector/actor-inspector";
import type { Registry } from "@/mod";
import {
	ACTOR_VERSIONED,
	CONN_VERSIONED,
} from "@/schemas/actor-persist/versioned";
import { EXTRA_ERROR_LOG } from "@/utils";
import type { ActorConfig } from "../config";
import type { ConnDriver } from "../conn/driver";
import { createHttpDriver } from "../conn/drivers/http";
import {
	CONN_DRIVER_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
	type Conn,
	type ConnId,
} from "../conn/mod";
import {
	convertConnFromBarePersistedConn,
	type PersistedConn,
} from "../conn/persisted";
import {
	type ActionContext,
	ActorContext,
	RequestContext,
	WebSocketContext,
} from "../contexts";
import type { AnyDatabaseProvider, InferDatabaseClient } from "../database";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { serializeActorKey } from "../keys";
import { processMessage } from "../protocol/old";
import { Schedule } from "../schedule";
import {
	assertUnreachable,
	DeadlineError,
	deadline,
	generateSecureToken,
} from "../utils";
import { ConnectionManager } from "./connection-manager";
import { EventManager } from "./event-manager";
import { KEYS } from "./keys";
import {
	convertActorFromBarePersisted,
	type PersistedActor,
} from "./persisted";
import { ScheduleManager } from "./schedule-manager";
import { type SaveStateOptions, StateManager } from "./state-manager";

export type { SaveStateOptions };

enum CanSleep {
	Yes,
	NotReady,
	NotStarted,
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
	driver!: ActorDriver;
	#inlineClient!: Client<Registry<any>>;
	#actorId!: string;
	#name!: string;
	#key!: ActorKey;
	#region!: string;

	// MARK: - Managers
	connectionManager!: ConnectionManager<S, CP, CS, V, I, DB>;

	stateManager!: StateManager<S, CP, CS, I>;

	eventManager!: EventManager<S, CP, CS, V, I, DB>;

	#scheduleManager!: ScheduleManager<S, CP, CS, V, I, DB>;

	// MARK: - Logging
	#log!: Logger;
	#rLog!: Logger;

	// MARK: - Lifecycle State
	/**
	 * If the core actor initiation has set up.
	 *
	 * Almost all actions on this actor will throw an error if false.
	 **/
	#ready = false;
	/**
	 * If the actor has fully started.
	 *
	 * The only purpose of this is to prevent sleeping until started.
	 */
	#started = false;
	#sleepCalled = false;
	#destroyCalled = false;
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
	#inspector = new ActorInspector(this);

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
		return this.connectionManager.connections;
	}

	get schedule(): Schedule {
		return this.#schedule;
	}

	get abortSignal(): AbortSignal {
		return this.#abortController.signal;
	}

	get actions(): string[] {
		return Object.keys(this.#config.actions ?? {});
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB> {
		return this.#config;
	}

	// MARK: - State Access
	get persist(): PersistedActor<S, I> {
		return this.stateManager.persist;
	}

	get state(): S {
		return this.stateManager.state;
	}

	set state(value: S) {
		this.stateManager.state = value;
	}

	get stateEnabled(): boolean {
		return this.stateManager.stateEnabled;
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
		this.driver = actorDriver;
		this.#inlineClient = inlineClient;
		this.#actorId = actorId;
		this.#name = name;
		this.#key = key;
		this.#region = region;

		// Initialize logging
		this.#initializeLogging();

		// Initialize managers
		this.connectionManager = new ConnectionManager(this);
		this.stateManager = new StateManager(this, actorDriver, this.#config);
		this.eventManager = new EventManager(this);
		this.#scheduleManager = new ScheduleManager(
			this,
			actorDriver,
			this.#config,
		);

		// Legacy schedule object (for compatibility)
		this.#schedule = new Schedule(this);

		// Load state
		await this.#loadState();

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

		// Finish up any remaining initiation
		//
		// Do this after #ready = true since this can call any actor callbacks
		// (which require #assertReady)
		await this.driver.onBeforeActorStart?.(this);

		// Mark as started
		//
		// We do this after onBeforeActorStart to prevent the actor from going
		// to sleep before finishing setup
		this.#started = true;
		this.#rLog.info({ msg: "actor started" });

		// Start sleep timer after setting #started since this affects the
		// timer
		this.resetSleepTimer();

		// Trigger any pending alarms
		await this.onAlarm();
	}

	// MARK: - Ready Check
	isReady(): boolean {
		return this.#ready;
	}

	assertReady(allowStoppingState: boolean = false) {
		if (!this.#ready) throw new errors.InternalError("Actor not ready");
		if (!allowStoppingState && this.#stopCalled)
			throw new errors.InternalError("Actor is stopping");
	}

	// MARK: - Stop
	async onStop(mode: "sleep" | "destroy") {
		if (this.#stopCalled) {
			this.#rLog.warn({ msg: "already stopping actor" });
			return;
		}
		this.#stopCalled = true;
		this.#rLog.info({
			msg: "setting stopCalled=true",
			mode,
		});

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
		if (mode === "sleep") {
			await this.#callOnSleep();
		} else if (mode === "destroy") {
			await this.#callOnDestroy();
		} else {
			assertUnreachable(mode);
		}

		// Disconnect non-hibernatable connections
		await this.#disconnectConnections();

		// Wait for background tasks
		await this.#waitBackgroundPromises(
			this.#config.options.waitUntilTimeout,
		);

		// Clear timeouts and save state
		this.#rLog.info({ msg: "clearing pending save timeouts" });
		this.stateManager.clearPendingSaveTimeout();
		this.#rLog.info({ msg: "saving state immediately" });
		await this.stateManager.saveState({
			immediate: true,
			allowStoppingState: true,
		});

		// Wait for write queues
		await this.stateManager.waitForPendingWrites();
		await this.#scheduleManager.waitForPendingAlarmWrites();
	}

	// MARK: - Sleep
	startSleep() {
		if (this.#stopCalled || this.#destroyCalled) {
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

		const sleep = this.driver.startSleep?.bind(this.driver, this.#actorId);
		invariant(this.#sleepingSupported, "sleeping not supported");
		invariant(sleep, "no sleep on driver");

		this.#rLog.info({ msg: "actor sleeping" });

		// Start sleep on next tick so call site of startSleep can exit
		setImmediate(() => {
			sleep();
		});
	}

	// MARK: - Destroy
	startDestroy() {
		if (this.#stopCalled || this.#sleepCalled) {
			this.#rLog.debug({
				msg: "cannot call startDestroy if actor already stopping or sleeping",
			});
			return;
		}

		if (this.#destroyCalled) {
			this.#rLog.warn({
				msg: "cannot call startDestroy twice, actor already destroying",
			});
			return;
		}
		this.#destroyCalled = true;

		const destroy = this.driver.startDestroy.bind(
			this.driver,
			this.#actorId,
		);

		this.#rLog.info({ msg: "actor destroying" });

		// Start destroy on next tick so call site of startDestroy can exit
		setImmediate(() => {
			destroy();
		});
	}

	// MARK: - HTTP Request Tracking
	beginHonoHttpRequest() {
		this.#activeHonoHttpRequests++;
		this.resetSleepTimer();
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
		this.resetSleepTimer();
	}

	// MARK: - Message Processing
	async processMessage(
		message: {
			body:
				| {
						tag: "ActionRequest";
						val: { id: bigint; name: string; args: unknown };
				  }
				| {
						tag: "SubscriptionRequest";
						val: { eventName: string; subscribe: boolean };
				  };
		},
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
				this.eventManager.addSubscription(eventName, conn, false);
			},
			onUnsubscribe: async (eventName, conn) => {
				this.inspector.emitter.emit("eventFired", {
					type: "unsubscribe",
					eventName,
					connId: conn.id,
				});
				this.eventManager.removeSubscription(eventName, conn, false);
			},
		});
	}

	// MARK: - Action Execution
	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		this.assertReady();

		const actions = this.#config.actions ?? {};
		if (!(actionName in actions)) {
			this.#rLog.warn({ msg: "action does not exist", actionName });
			throw new errors.ActionNotFound(actionName);
		}

		const actionFunction = actions[actionName];
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
			this.stateManager.savePersistThrottled();
		}
	}

	// MARK: - HTTP/WebSocket Handlers
	async handleRawRequest(
		conn: Conn<S, CP, CS, V, I, DB>,
		request: Request,
	): Promise<Response> {
		this.assertReady();

		if (!this.#config.onRequest) {
			throw new errors.RequestHandlerNotDefined();
		}

		try {
			const ctx = new RequestContext(this, conn, request);
			const response = await this.#config.onRequest(ctx, request);
			if (!response) {
				throw new errors.InvalidRequestHandlerResponse();
			}
			return response;
		} catch (error) {
			this.#rLog.error({
				msg: "onRequest error",
				error: stringifyError(error),
			});
			throw error;
		} finally {
			this.stateManager.savePersistThrottled();
		}
	}

	handleRawWebSocket(
		conn: Conn<S, CP, CS, V, I, DB>,
		websocket: UniversalWebSocket,
		request?: Request,
	) {
		// NOTE: All code before `onWebSocket` must be synchronous in order to ensure the order of `open` events happen in the correct order.

		this.assertReady();

		if (!this.#config.onWebSocket) {
			throw new errors.InternalError("onWebSocket handler not defined");
		}

		try {
			// Reset sleep timer when handling WebSocket
			this.resetSleepTimer();

			// Handle WebSocket
			const ctx = new WebSocketContext(this, conn, request);

			// NOTE: This is async and will run in the background
			const voidOrPromise = this.#config.onWebSocket(ctx, websocket);

			// Save changes from the WebSocket open
			if (voidOrPromise instanceof Promise) {
				voidOrPromise.then(() => {
					this.stateManager.savePersistThrottled();
				});
			} else {
				this.stateManager.savePersistThrottled();
			}
		} catch (error) {
			this.#rLog.error({
				msg: "onWebSocket error",
				error: stringifyError(error),
			});
			throw error;
		}
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
		this.resetSleepTimer();
		await this.#scheduleManager.onAlarm();
	}

	// MARK: - Background Tasks
	waitUntil(promise: Promise<void>) {
		this.assertReady();

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

		const extraLogParams = this.driver.getExtraActorLogParams?.();
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

	async #loadState() {
		// Read initial state from KV
		const [persistDataBuffer] = await this.driver.kvBatchGet(
			this.#actorId,
			[KEYS.PERSIST_DATA],
		);
		invariant(
			persistDataBuffer !== null,
			"persist data has not been set, it should be set when initialized",
		);

		const bareData =
			ACTOR_VERSIONED.deserializeWithEmbeddedVersion(persistDataBuffer);
		const persistData = convertActorFromBarePersisted<S, I>(bareData);

		if (persistData.hasInitialized) {
			// Restore existing actor
			await this.#restoreExistingActor(persistData);
		} else {
			// Create new actor
			await this.#createNewActor(persistData);
		}

		// Pass persist reference to schedule manager
		this.#scheduleManager.setPersist(this.stateManager.persist);
	}

	async #createNewActor(persistData: PersistedActor<S, I>) {
		this.#rLog.info({ msg: "actor creating" });

		// Initialize state
		await this.stateManager.initializeState(persistData);

		// Call onCreate lifecycle
		if (this.#config.onCreate) {
			await this.#config.onCreate(
				this.actorContext as any,
				persistData.input!,
			);
		}
	}

	async #restoreExistingActor(persistData: PersistedActor<S, I>) {
		// List all connection keys
		const connEntries = await this.driver.kvListPrefix(
			this.#actorId,
			KEYS.CONN_PREFIX,
		);

		// Decode connections
		const connections: PersistedConn<CP, CS>[] = [];
		for (const [_key, value] of connEntries) {
			try {
				const bareData = CONN_VERSIONED.deserializeWithEmbeddedVersion(
					new Uint8Array(value),
				);
				const conn = convertConnFromBarePersistedConn<CP, CS>(bareData);
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
		});

		// Initialize state
		this.stateManager.initPersistProxy(persistData);

		// Restore connections
		this.connectionManager.restoreConnections(connections);
	}

	async #initializeInspectorToken() {
		// Try to load existing token
		const [tokenBuffer] = await this.driver.kvBatchGet(this.#actorId, [
			KEYS.INSPECTOR_TOKEN,
		]);

		if (tokenBuffer !== null) {
			// Token exists, decode it
			const decoder = new TextDecoder();
			this.#inspectorToken = decoder.decode(tokenBuffer);
			this.#rLog.debug({ msg: "loaded existing inspector token" });
		} else {
			// Generate new token
			this.#inspectorToken = generateSecureToken();
			const tokenBytes = new TextEncoder().encode(this.#inspectorToken);
			await this.driver.kvBatchPut(this.#actorId, [
				[KEYS.INSPECTOR_TOKEN, tokenBytes],
			]);
			this.#rLog.debug({ msg: "generated new inspector token" });
		}
	}

	async #initializeVars() {
		let vars: V | undefined;
		if ("createVars" in this.#config) {
			const dataOrPromise = this.#config.createVars(
				this.actorContext as any,
				this.driver.getContext(this.#actorId),
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

	async #callOnSleep() {
		if (this.#config.onSleep) {
			try {
				this.#rLog.debug({ msg: "calling onSleep" });
				const result = this.#config.onSleep(this.actorContext);
				if (result instanceof Promise) {
					await deadline(result, this.#config.options.onSleepTimeout);
				}
				this.#rLog.debug({ msg: "onSleep completed" });
			} catch (error) {
				if (error instanceof DeadlineError) {
					this.#rLog.error({ msg: "onSleep timed out" });
				} else {
					this.#rLog.error({
						msg: "error in onSleep",
						error: stringifyError(error),
					});
				}
			}
		}
	}

	async #callOnDestroy() {
		if (this.#config.onDestroy) {
			try {
				this.#rLog.debug({ msg: "calling onDestroy" });
				const result = this.#config.onDestroy(this.actorContext);
				if (result instanceof Promise) {
					await deadline(
						result,
						this.#config.options.onDestroyTimeout,
					);
				}
				this.#rLog.debug({ msg: "onDestroy completed" });
			} catch (error) {
				if (error instanceof DeadlineError) {
					this.#rLog.error({ msg: "onDestroy timed out" });
				} else {
					this.#rLog.error({
						msg: "error in onDestroy",
						error: stringifyError(error),
					});
				}
			}
		}
	}

	async #setupDatabase() {
		if ("db" in this.#config && this.#config.db) {
			const client = await this.#config.db.createClient({
				getDatabase: () => this.driver.getDatabase(this.#actorId),
			});
			this.#rLog.info({ msg: "database migration starting" });
			await this.#config.db.onMigrate?.(client);
			this.#rLog.info({ msg: "database migration complete" });
			this.#db = client;
		}
	}

	async #disconnectConnections() {
		const promises: Promise<unknown>[] = [];
		this.#rLog.debug({
			msg: "disconnecting connections on actor stop",
			totalConns: this.connectionManager.connections.size,
		});
		for (const connection of this.connectionManager.connections.values()) {
			this.#rLog.debug({
				msg: "checking connection for disconnect",
				connId: connection.id,
				isHibernatable: connection.isHibernatable,
			});
			if (!connection.isHibernatable) {
				this.#rLog.debug({
					msg: "disconnecting non-hibernatable connection on actor stop",
					connId: connection.id,
				});
				promises.push(connection.disconnect());
			} else {
				this.#rLog.debug({
					msg: "preserving hibernatable connection on actor stop",
					connId: connection.id,
				});
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

	resetSleepTimer() {
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
		if (!this.#started) return CanSleep.NotReady;
		if (this.#activeHonoHttpRequests > 0)
			return CanSleep.ActiveHonoHttpRequests;

		for (const _conn of this.connectionManager.connections.values()) {
			// TODO: Add back
			// if (!_conn.isHibernatable) {
			return CanSleep.ActiveConns;
			// }
		}

		return CanSleep.Yes;
	}

	get #sleepingSupported(): boolean {
		return this.driver.startSleep !== undefined;
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
