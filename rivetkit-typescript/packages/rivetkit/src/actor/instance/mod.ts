import type { OtlpExportTraceServiceRequestJson } from "@rivetkit/traces";
import {
	createTraces,
	type SpanHandle,
	type SpanStatusInput,
	type Traces,
} from "@rivetkit/traces";
import type { SqliteVfs } from "@rivetkit/sqlite-vfs";
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
import {
	type ActorConfig,
	getRunFunction,
} from "../config";
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
	ActionContext,
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
	type EventSchemaConfig,
	getEventCanSubscribe,
	getQueueCanPublish,
	type QueueSchemaConfig,
} from "../schema";
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
import { QueueManager } from "./queue-manager";
import { ScheduleManager } from "./schedule-manager";
import { type SaveStateOptions, StateManager } from "./state-manager";
import { ActorTracesDriver } from "./traces-driver";

export type { SaveStateOptions };

enum CanSleep {
	Yes,
	NotReady,
	NotStarted,
	ActiveConns,
	ActiveDisconnectCallbacks,
	ActiveHonoHttpRequests,
	ActiveKeepAwake,
	ActiveRun,
}

/** Actor type alias with all `any` types. Used for `extends` in classes referencing this actor. */
export type AnyActorInstance = ActorInstance<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export type ExtractActorState<A extends AnyActorInstance> =
	A extends ActorInstance<infer State, any, any, any, any, any, any, any>
		? State
		: never;

export type ExtractActorConnParams<A extends AnyActorInstance> =
	A extends ActorInstance<any, infer ConnParams, any, any, any, any, any, any>
		? ConnParams
		: never;

export type ExtractActorConnState<A extends AnyActorInstance> =
	A extends ActorInstance<any, any, infer ConnState, any, any, any, any, any>
		? ConnState
		: never;

// MARK: - Main ActorInstance Class
export class ActorInstance<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
> {
	// MARK: - Core Properties
	actorContext: ActorContext<S, CP, CS, V, I, DB, E, Q>;
	#config: ActorConfig<S, CP, CS, V, I, DB, E, Q>;
	driver!: ActorDriver;
	#inlineClient!: Client<Registry<any>>;
	#actorId!: string;
	#name!: string;
	#key!: ActorKey;
	#actorKeyString!: string;
	#region!: string;

	// MARK: - Managers
	connectionManager!: ConnectionManager<S, CP, CS, V, I, DB, E, Q>;

	stateManager!: StateManager<S, CP, CS, I, E, Q>;

	eventManager!: EventManager<S, CP, CS, V, I, DB, E, Q>;

	#scheduleManager!: ScheduleManager<S, CP, CS, V, I, DB, E, Q>;

	queueManager!: QueueManager<S, CP, CS, V, I, DB, E, Q>;

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
	#db?: InferDatabaseClient<DB>;
	#sqliteVfs?: SqliteVfs;

	// MARK: - Background Tasks
	#backgroundPromises: Promise<void>[] = [];
	#runPromise?: Promise<void>;
	#runHandlerActive = false;
	#activeQueueWaitCount = 0;

	// MARK: - HTTP/WebSocket Tracking
	#activeHonoHttpRequests = 0;
	#activeKeepAwakeCount = 0;

	// MARK: - Deprecated (kept for compatibility)
	#schedule!: Schedule;

	// MARK: - Inspector
	#inspectorToken?: string;
	#inspector: ActorInspector;

	// MARK: - Tracing
	#traces!: Traces<OtlpExportTraceServiceRequestJson>;

	// MARK: - Constructor
	constructor(config: ActorConfig<S, CP, CS, V, I, DB, E, Q>) {
		this.#config = config;
		this.actorContext = new ActorContext(this);
		this.#inspector = new ActorInspector(this);
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

	get traces(): Traces<OtlpExportTraceServiceRequestJson> {
		return this.#traces;
	}

	get inspectorToken(): string | undefined {
		return this.#inspectorToken;
	}

	// MARK: - Tracing
	getCurrentTraceSpan(): SpanHandle | null {
		return this.#traces.getCurrentSpan();
	}

	startTraceSpan(
		name: string,
		attributes?: Record<string, unknown>,
	): SpanHandle {
		return this.#traces.startSpan(name, {
			parent: this.#traces.getCurrentSpan() ?? undefined,
			attributes: this.#traceAttributes(attributes),
		});
	}

	endTraceSpan(handle: SpanHandle, status?: SpanStatusInput): void {
		this.#traces.endSpan(handle, status ? { status } : undefined);
	}

	async runInTraceSpan<T>(
		name: string,
		attributes: Record<string, unknown> | undefined,
		fn: () => T | Promise<T>,
	): Promise<T> {
		const span = this.startTraceSpan(name, attributes);
		try {
			const result = this.#traces.withSpan(span, fn);
			const resolved = result instanceof Promise ? await result : result;
			this.#traces.endSpan(span, {
				status: { code: "OK" },
			});
			return resolved;
		} catch (error) {
			this.#traces.endSpan(span, {
				status: {
					code: "ERROR",
					message: stringifyError(error),
				},
			});
			throw error;
		}
	}

	emitTraceEvent(
		name: string,
		attributes?: Record<string, unknown>,
		handle?: SpanHandle,
	): void {
		const span = handle ?? this.#traces.getCurrentSpan();
		if (!span) {
			return;
		}
		this.#traces.emitEvent(span, name, {
			attributes: this.#traceAttributes(attributes),
			timeUnixMs: Date.now(),
		});
	}

	get conns(): Map<ConnId, Conn<S, CP, CS, V, I, DB, E, Q>> {
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

	get config(): ActorConfig<S, CP, CS, V, I, DB, E, Q> {
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
		this.#actorKeyString = serializeActorKey(this.#key);
		this.#region = region;

		// Initialize tracing
		this.#initializeTraces();

		// Initialize logging
		this.#initializeLogging();

		// Initialize managers
		this.connectionManager = new ConnectionManager(this);
		this.stateManager = new StateManager(this, actorDriver, this.#config);
		this.eventManager = new EventManager(this);
		this.queueManager = new QueueManager(this, actorDriver);
		this.#scheduleManager = new ScheduleManager(
			this,
			actorDriver,
			this.#config,
		);

		// Legacy schedule object (for compatibility)
		this.#schedule = new Schedule(this);

		// Load state
		await this.#loadState();

		await this.queueManager.initialize();

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

		// Start run handler in background (does not block startup)
		this.#startRunHandler();

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

	async cleanupPersistedConnections(reason?: string): Promise<number> {
		this.assertReady(true);
		return await this.connectionManager.cleanupPersistedHibernatableConnections(
			reason,
		);
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

		try {
			// Clear sleep timeout
			if (this.#sleepTimeout) {
				clearTimeout(this.#sleepTimeout);
				this.#sleepTimeout = undefined;
			}

			// Abort listeners
			try {
				this.#abortController.abort();
			} catch { }

			// Wait for run handler to complete
			await this.#waitForRunHandler(this.#config.options.runStopTimeout);

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
		} finally {
			await this.#cleanupDatabase();
		}
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
		conn: Conn<S, CP, CS, V, I, DB, E, Q>,
	) {
		await processMessage(message, this, conn, {
			onExecuteAction: async (ctx, name, args) => {
				return await this.executeAction(ctx, name, args);
			},
			onSubscribe: async (eventName, conn) => {
				this.eventManager.addSubscription(eventName, conn, false);
			},
			onUnsubscribe: async (eventName, conn) => {
				this.eventManager.removeSubscription(eventName, conn, false);
			},
		});
	}

	async assertCanSubscribe(
		ctx: ActionContext<S, CP, CS, V, I, DB, E, Q>,
		eventName: string,
	): Promise<void> {
		const canSubscribe = getEventCanSubscribe(this.#config.events, eventName);
		if (!canSubscribe) {
			return;
		}

		const result = await canSubscribe(ctx);
		if (typeof result !== "boolean") {
			throw new errors.InvalidCanSubscribeResponse();
		}
		if (!result) {
			throw new errors.Forbidden();
		}
	}

	async assertCanPublish(
		ctx: ActionContext<S, CP, CS, V, I, DB, E, Q>,
		queueName: string,
	): Promise<void> {
		const canPublish = getQueueCanPublish<
			ActionContext<S, CP, CS, V, I, DB, E, Q>
		>(this.#config.queues, queueName);
		if (!canPublish) {
			return;
		}

		const result = await canPublish(ctx);
		if (typeof result !== "boolean") {
			throw new errors.InvalidCanPublishResponse();
		}
		if (!result) {
			throw new errors.Forbidden();
		}
	}

	// MARK: - Action Execution
	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB, E, Q>,
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

		this.#activeKeepAwakeCount++;
		this.resetSleepTimer();
		const actionSpan = this.startTraceSpan(`actor.action.${actionName}`, {
			"rivet.action.name": actionName,
		});
		let spanEnded = false;

		try {
			return await this.#traces.withSpan(actionSpan, async () => {
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
				const maybeThenable = outputOrPromise as {
					then?: (
						onfulfilled?: unknown,
						onrejected?: unknown,
					) => unknown;
				};
				if (maybeThenable && typeof maybeThenable.then === "function") {
					output = await deadline(
						Promise.resolve(outputOrPromise),
						this.#config.options.actionTimeout,
					);
				} else {
					output = outputOrPromise;
				}

				// Process through onBeforeActionResponse if configured
				if (this.#config.onBeforeActionResponse) {
					try {
						output = await this.runInTraceSpan(
							"actor.onBeforeActionResponse",
							{ "rivet.action.name": actionName },
							() =>
								this.#config.onBeforeActionResponse!(
									this.actorContext,
									actionName,
									args,
									output,
								),
						);
					} catch (error) {
						this.#rLog.error({
							msg: "error in `onBeforeActionResponse`",
							error: stringifyError(error),
						});
					}
				}

				return output;
			});
		} catch (error) {
			const isTimeout = error instanceof DeadlineError;
			const message = isTimeout
				? "ActionTimedOut"
				: stringifyError(error);
			this.#traces.setAttributes(actionSpan, {
				"error.message": message,
				"error.type":
					error instanceof Error ? error.name : typeof error,
			});
			this.#traces.endSpan(actionSpan, {
				status: { code: "ERROR", message },
			});
			spanEnded = true;
			if (isTimeout) {
				throw new errors.ActionTimedOut();
			}
			this.#rLog.error({
				msg: "action error",
				actionName,
				error: stringifyError(error),
			});
			throw error;
		} finally {
			if (!spanEnded && actionSpan.isActive()) {
				this.#traces.endSpan(actionSpan, {
					status: { code: "OK" },
				});
			}
			this.#activeKeepAwakeCount--;
			if (this.#activeKeepAwakeCount < 0) {
				this.#activeKeepAwakeCount = 0;
				this.#rLog.warn({
					msg: "active keep awake count went below 0, this is a RivetKit bug",
					...EXTRA_ERROR_LOG,
				});
			}
			this.resetSleepTimer();
			this.stateManager.savePersistThrottled();
		}
	}

	// MARK: - HTTP/WebSocket Handlers
	async handleRawRequest(
		conn: Conn<S, CP, CS, V, I, DB, E, Q>,
		request: Request,
	): Promise<Response> {
		this.assertReady();

		if (!this.#config.onRequest) {
			throw new errors.RequestHandlerNotDefined();
		}
		const onRequest = this.#config.onRequest;

			return await this.runInTraceSpan(
				"actor.onRequest",
				{
					"http.method": request.method,
					"http.url": request.url,
					"rivet.conn.id": conn.id,
				},
				async () => {
					const ctx = new RequestContext(this, conn, request);
					try {
						const response = await onRequest(ctx, request);
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
				},
			);
		}

	handleRawWebSocket(
		conn: Conn<S, CP, CS, V, I, DB, E, Q>,
		websocket: UniversalWebSocket,
		request?: Request,
	) {
		// NOTE: All code before `onWebSocket` must be synchronous in order to ensure the order of `open` events happen in the correct order.

		this.assertReady();

		if (!this.#config.onWebSocket) {
			throw new errors.InternalError("onWebSocket handler not defined");
		}

		const span = this.startTraceSpan("actor.onWebSocket", {
			"http.url": request?.url,
			"rivet.conn.id": conn.id,
		});
		let spanEnded = false;

		try {
			// Reset sleep timer when handling WebSocket
			this.resetSleepTimer();

			// Handle WebSocket
			const ctx = new WebSocketContext(this, conn, request);

			// NOTE: This is async and will run in the background
			const voidOrPromise = this.#traces.withSpan(span, () =>
				this.#config.onWebSocket!(ctx, websocket),
			);

			// Save changes from the WebSocket open
			if (voidOrPromise instanceof Promise) {
				voidOrPromise
					.then(() => {
						if (!spanEnded) {
							this.endTraceSpan(span, { code: "OK" });
							spanEnded = true;
						}
					})
					.catch((error) => {
						if (!spanEnded) {
							this.endTraceSpan(span, {
								code: "ERROR",
								message: stringifyError(error),
							});
							spanEnded = true;
						}
						this.#rLog.error({
							msg: "onWebSocket error",
							error: stringifyError(error),
						});
					})
					.finally(() => {
						this.stateManager.savePersistThrottled();
					});
			} else {
				if (!spanEnded) {
					this.endTraceSpan(span, { code: "OK" });
					spanEnded = true;
				}
				this.stateManager.savePersistThrottled();
			}
		} catch (error) {
			if (!spanEnded) {
				this.endTraceSpan(span, {
					code: "ERROR",
					message: stringifyError(error),
				});
				spanEnded = true;
			}
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

	/**
	 * Prevents the actor from sleeping while the given promise is running.
	 *
	 * Use this when performing async operations in the `run` handler or other
	 * background contexts where you need to ensure the actor stays awake.
	 *
	 * Returns the resolved value and resets the sleep timer on completion.
	 * Errors are propagated to the caller.
	 */
	async keepAwake<T>(promise: Promise<T>): Promise<T> {
		this.assertReady();

		this.#activeKeepAwakeCount++;
		this.resetSleepTimer();

		try {
			return await promise;
		} finally {
			this.#activeKeepAwakeCount--;
			if (this.#activeKeepAwakeCount < 0) {
				this.#activeKeepAwakeCount = 0;
				this.#rLog.warn({
					msg: "active keep awake count went below 0, this is a RivetKit bug",
					...EXTRA_ERROR_LOG,
				});
			}
			this.resetSleepTimer();
		}
	}

	beginQueueWait() {
		this.assertReady(true);
		this.#activeQueueWaitCount++;
		this.resetSleepTimer();
	}

	endQueueWait() {
		this.#activeQueueWaitCount--;
		if (this.#activeQueueWaitCount < 0) {
			this.#activeQueueWaitCount = 0;
			this.#rLog.warn({
				msg: "active queue wait count went below 0, this is a RivetKit bug",
				...EXTRA_ERROR_LOG,
			});
		}
		this.resetSleepTimer();
	}

	// MARK: - Private Helper Methods
	#initializeTraces() {
		this.#traces = createTraces({
			driver: new ActorTracesDriver(this.driver, this.#actorId),
		});
	}

	#traceAttributes(
		attributes?: Record<string, unknown>,
	): Record<string, unknown> {
		return {
			"rivet.actor.id": this.#actorId,
			"rivet.actor.name": this.#name,
			"rivet.actor.key": this.#actorKeyString,
			"rivet.actor.region": this.#region,
			...(attributes ?? {}),
		};
	}

	#patchLoggerForTraces(logger: Logger) {
		const levels: Array<
			"trace" | "debug" | "info" | "warn" | "error" | "fatal"
		> = ["trace", "debug", "info", "warn", "error", "fatal"];
		for (const level of levels) {
			const original = logger[level].bind(logger) as (
				...args: any[]
			) => unknown;
			logger[level] = ((...args: unknown[]) => {
				this.#emitLogEvent(level, args);
				return original(...(args as any[]));
			}) as Logger[typeof level];
		}
	}

	#emitLogEvent(level: string, args: unknown[]) {
		const span = this.#traces.getCurrentSpan();
		if (!span || !span.isActive()) {
			return;
		}

		let message: string | undefined;
		if (args.length >= 2) {
			message = String(args[1]);
		} else if (args.length === 1) {
			const [value] = args;
			if (typeof value === "string") {
				message = value;
			} else if (
				typeof value === "number" ||
				typeof value === "boolean"
			) {
				message = String(value);
			} else if (value && typeof value === "object") {
				const maybeMsg = (value as { msg?: unknown }).msg;
				if (maybeMsg !== undefined) {
					message = String(maybeMsg);
				}
			}
		}

		this.#traces.emitEvent(span, "log", {
			attributes: this.#traceAttributes({
				"log.level": level,
				...(message ? { "log.message": message } : {}),
			}),
			timeUnixMs: Date.now(),
		});
	}

	#initializeLogging() {
		const logParams = {
			actor: this.#name,
			key: this.#actorKeyString,
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

		this.#patchLoggerForTraces(this.#log);
		this.#patchLoggerForTraces(this.#rLog);
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
			const onCreate = this.#config.onCreate;
			await this.runInTraceSpan("actor.onCreate", undefined, () =>
				onCreate(this.actorContext as any, persistData.input!),
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
			const createVars = this.#config.createVars;
			vars = await this.runInTraceSpan(
				"actor.createVars",
				undefined,
				() => {
					const dataOrPromise = createVars!(
						this.actorContext as any,
						this.driver.getContext(this.#actorId),
					);
					if (dataOrPromise instanceof Promise) {
						return deadline(
							dataOrPromise,
							this.#config.options.createVarsTimeout,
						);
					}
					return dataOrPromise;
				},
			);
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
			const onWake = this.#config.onWake;
			await this.runInTraceSpan("actor.onWake", undefined, () =>
				onWake(this.actorContext),
			);
		}
	}

	async #callOnSleep() {
		if (this.#config.onSleep) {
			const onSleep = this.#config.onSleep;
			try {
				this.#rLog.debug({ msg: "calling onSleep" });
				await this.runInTraceSpan(
					"actor.onSleep",
					undefined,
					async () => {
						const result = onSleep(this.actorContext);
						if (result instanceof Promise) {
							await deadline(
								result,
								this.#config.options.onSleepTimeout,
							);
						}
					},
				);
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
			const onDestroy = this.#config.onDestroy;
			try {
				this.#rLog.debug({ msg: "calling onDestroy" });
				await this.runInTraceSpan(
					"actor.onDestroy",
					undefined,
					async () => {
						const result = onDestroy(this.actorContext);
						if (result instanceof Promise) {
							await deadline(
								result,
								this.#config.options.onDestroyTimeout,
							);
						}
					},
				);
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

	#startRunHandler() {
		const runFn = getRunFunction(this.#config.run);
		if (!runFn) return;

		this.#rLog.debug({ msg: "starting run handler" });
		this.#runHandlerActive = true;
		this.resetSleepTimer();

		const runSpan = this.startTraceSpan("actor.run");
		const runResult = this.#traces.withSpan(runSpan, () =>
			runFn(this.actorContext),
		);

		if (runResult instanceof Promise) {
			this.#runPromise = runResult
				.then(() => {
					if (this.#stopCalled) {
						if (runSpan.isActive()) {
							this.endTraceSpan(runSpan, { code: "OK" });
						}
						this.#rLog.debug({
							msg: "run handler exited during actor stop",
						});
						return;
					}

					// Run handler exited normally - this should crash the actor
					this.emitTraceEvent(
						"actor.crash",
						{ "rivet.actor.reason": "run_exited" },
						runSpan,
					);
					this.endTraceSpan(runSpan, {
						code: "ERROR",
						message: "run exited unexpectedly",
					});
					this.#rLog.warn({
						msg: "run handler exited unexpectedly, crashing actor to reschedule",
					});
					this.startDestroy();
				})
				.catch((error) => {
					if (this.#stopCalled) {
						if (runSpan.isActive()) {
							this.endTraceSpan(runSpan, { code: "OK" });
						}
						this.#rLog.debug({
							msg: "run handler threw during actor stop",
							error: stringifyError(error),
						});
						return;
					}

					// Run handler threw an error - crash the actor
					this.emitTraceEvent(
						"actor.crash",
						{
							"rivet.actor.reason": "run_error",
							"error.message": stringifyError(error),
						},
						runSpan,
					);
					this.endTraceSpan(runSpan, {
						code: "ERROR",
						message: stringifyError(error),
					});
					this.#rLog.error({
						msg: "run handler threw error, crashing actor to reschedule",
						error: stringifyError(error),
					});
					this.startDestroy();
				})
				.finally(() => {
					this.#runHandlerActive = false;
					this.resetSleepTimer();
				});
		} else if (runSpan.isActive()) {
			this.endTraceSpan(runSpan, { code: "OK" });
			this.#runHandlerActive = false;
			this.resetSleepTimer();
		}
	}

	async #waitForRunHandler(timeoutMs: number) {
		if (!this.#runPromise) {
			return;
		}

		this.#rLog.debug({ msg: "waiting for run handler to complete" });

		const timedOut = await Promise.race([
			this.#runPromise.then(() => false).catch(() => false),
			new Promise<true>((resolve) =>
				setTimeout(() => resolve(true), timeoutMs),
			),
		]);

		if (timedOut) {
			this.#rLog.warn({
				msg: "run handler did not complete in time, it may have leaked - ensure you use c.aborted (or the abort signal c.abortSignal) to exit gracefully",
				timeoutMs,
			});
		} else {
			this.#rLog.debug({ msg: "run handler completed" });
		}
	}

	async #setupDatabase() {
		if (!("db" in this.#config) || !this.#config.db) {
			return;
		}

		let client: InferDatabaseClient<DB> | undefined;
		try {
			// Every actor gets its own SqliteVfs/wa-sqlite instance. The async
			// wa-sqlite build is not re-entrant, and sharing one instance across
			// actors can cause cross-actor contention and runtime corruption.
			this.#sqliteVfs ??= this.driver.sqliteVfs;

			client = await this.#config.db.createClient({
				actorId: this.#actorId,
				overrideRawDatabaseClient: this.driver.overrideRawDatabaseClient
					? () => this.driver.overrideRawDatabaseClient!(this.#actorId)
					: undefined,
				overrideDrizzleDatabaseClient: this.driver.overrideDrizzleDatabaseClient
					? () => this.driver.overrideDrizzleDatabaseClient!(this.#actorId)
					: undefined,
				kv: {
					batchPut: (entries) => this.driver.kvBatchPut(this.#actorId, entries),
					batchGet: (keys) => this.driver.kvBatchGet(this.#actorId, keys),
					batchDelete: (keys) => this.driver.kvBatchDelete(this.#actorId, keys),
				},
				sqliteVfs: this.#sqliteVfs,
			});
			this.#rLog.info({ msg: "database migration starting" });
			await this.#config.db.onMigrate?.(client);
			this.#rLog.info({ msg: "database migration complete" });
			this.#db = client;
		} catch (error) {
			if (client) {
				try {
					await this.#config.db.onDestroy?.(client);
				} catch (cleanupError) {
					this.#rLog.error({
						msg: "database setup cleanup failed",
						error: stringifyError(cleanupError),
					});
				}
			}
			this.#sqliteVfs = undefined;
			if (error instanceof Error) {
				this.#rLog.error({
					msg: "database setup failed",
					error: stringifyError(error),
				});
				throw error;
			}
			const wrappedError = new Error(`Database setup failed: ${String(error)}`);
			this.#rLog.error({
				msg: "database setup failed with non-Error object",
				error: String(error),
				errorType: typeof error,
			});
			throw wrappedError;
		}
	}

	async #cleanupDatabase() {
		const client = this.#db;
		this.#db = undefined;
		this.#sqliteVfs = undefined;

		if (!client) {
			return;
		}
		if (!("db" in this.#config) || !this.#config.db) {
			return;
		}

		try {
			await this.#config.db.onDestroy?.(client);
		} catch (error) {
			this.#rLog.error({
				msg: "database cleanup failed",
				error: stringifyError(error),
			});
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
		if (this.#activeKeepAwakeCount > 0) return CanSleep.ActiveKeepAwake;
		if (this.#runHandlerActive && this.#activeQueueWaitCount === 0) {
			return CanSleep.ActiveRun;
		}

		for (const _conn of this.connectionManager.connections.values()) {
			// TODO: Add back
			// if (!_conn.isHibernatable) {
			return CanSleep.ActiveConns;
			// }
		}

		if (this.connectionManager.pendingDisconnectCount > 0) {
			return CanSleep.ActiveDisconnectCallbacks;
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
