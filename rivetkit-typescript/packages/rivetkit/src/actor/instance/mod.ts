import type { OtlpExportTraceServiceRequestJson } from "@rivetkit/traces";
import {
	createNoopTraces,
	createTraces,
	type SpanHandle,
	type SpanStatusInput,
	type Traces,
} from "@rivetkit/traces";
import { ActorMetrics, type StartupTimingKey } from "@/actor/metrics";
import invariant from "invariant";
import type { Client } from "@/client/client";
import { getBaseLogger, getIncludeTarget, type Logger } from "@/common/log";
import { stringifyError } from "@/common/utils";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import { ActorInspector } from "@/inspector/actor-inspector";
import type { ActorKey } from "@/client/query";
import type { Registry } from "@/mod";
import {
	ACTOR_VERSIONED,
	CONN_VERSIONED,
} from "@/schemas/actor-persist/versioned";
import { EXTRA_ERROR_LOG } from "@/utils";
import { getRivetExperimentalOtel } from "@/utils/env-vars";
import { promiseWithResolvers } from "@/utils";
import {
	type Actions,
	type ActorConfig,
	type ActorConfigInput,
	ActorConfigSchema,
	DEFAULT_ON_SLEEP_TIMEOUT,
	DEFAULT_SLEEP_GRACE_PERIOD,
	DEFAULT_WAIT_UNTIL_TIMEOUT,
	getRunFunction,
} from "../config";
import type { ConnDriver } from "../conn/driver";
import { createHttpDriver } from "../conn/drivers/http";
import {
	HibernatableWebSocketAckState,
	handleInboundHibernatableWebSocketMessage as applyInboundHibernatableWebSocketMessage,
} from "../conn/hibernatable-websocket-ack-state";
import {
	CONN_DRIVER_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
	type AnyConn,
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
import { ActorDefinition } from "../definition";
import type { ActorDriver } from "../driver";
import * as errors from "../errors";
import { serializeActorKey } from "../keys";
import { getValueLength, processMessage } from "../protocol/old";
import type { InputData } from "../protocol/serde";
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
import { KEYS, workflowStoragePrefix } from "./keys";
import {
	type PreloadedEntries,
	type PreloadHit,
	type PreloadMap,
} from "./preload-map";
import {
	convertActorFromBarePersisted,
	type PersistedActor,
} from "./persisted";
import { QueueManager } from "./queue-manager";
import { ScheduleManager } from "./schedule-manager";
import { type SaveStateOptions, StateManager } from "./state-manager";
import { TrackedWebSocket } from "./tracked-websocket";
import { ActorTracesDriver } from "./traces-driver";
import { WriteCollector } from "./write-collector";

export type { SaveStateOptions };

/**
 * Symbol used by subsystems (e.g., queue-manager) to access the
 * unexpected KV round-trip warning without exposing it as a public method.
 */
export const WARN_UNEXPECTED_KV_ROUND_TRIP = Symbol(
	"warnUnexpectedKvRoundTrip",
);

export function actor<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> = Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues,
		TActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues,
	TActions
> {
	const config = ActorConfigSchema.parse(input) as ActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	return new ActorDefinition(config);
}

enum CanSleep {
	Yes,
	NotReady,
	NotStarted,
	PreventSleep,
	ActiveConns,
	ActiveDisconnectCallbacks,
	ActiveHonoHttpRequests,
	ActiveKeepAwake,
	ActiveInternalKeepAwake,
	ActiveRun,
	ActiveWebSocketCallbacks,
}

/**
 * Names of actor-managed async regions that should keep the actor awake while
 * work is still running.
 */
interface ActiveAsyncRegionCounts {
	keepAwake: number;
	internalKeepAwake: number;
	websocketCallbacks: number;
}

/**
 * Error messages for the async-region counters. These are used when a counter
 * underflows, which indicates mismatched begin/end bookkeeping.
 */
const ACTIVE_ASYNC_REGION_ERROR_MESSAGES: Record<
	keyof ActiveAsyncRegionCounts,
	string
> = {
	keepAwake: "active keep awake count went below 0, this is a RivetKit bug",
	internalKeepAwake:
		"active internal keep awake count went below 0, this is a RivetKit bug",
	websocketCallbacks:
		"active websocket callback count went below 0, this is a RivetKit bug",
};

/**
 * Minimal lifecycle contract shared by static and dynamic actor instances.
 *
 * Runtime internals (connections, inspector, queue manager, etc) are exposed
 * only on `ActorInstance`.
 */
export interface BaseActorInstance<
	S = any,
	CP = any,
	CS = any,
	V = any,
	I = any,
	DB extends AnyDatabaseProvider = AnyDatabaseProvider,
	E extends EventSchemaConfig = Record<never, never>,
	Q extends QueueSchemaConfig = Record<never, never>,
> {
	readonly id: string;
	readonly isStopping: boolean;
	onStop(mode: "sleep" | "destroy"): Promise<void>;
	onAlarm(): Promise<void>;
	cleanupPersistedConnections?(reason?: string): Promise<number>;
	getHibernatingWebSocketMetadata?(): Array<{
		gatewayId: ArrayBuffer;
		requestId: ArrayBuffer;
		serverMessageIndex: number;
		clientMessageIndex: number;
		path: string;
		headers: Record<string, string>;
	}>;
}

/** Actor type alias with all `any` types. */
export type AnyActorInstance = BaseActorInstance<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

/** Static actor type alias with all `any` types. */
export type AnyStaticActorInstance = ActorInstance<
	any,
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export function isStaticActorInstance(
	actor: AnyActorInstance,
): actor is AnyStaticActorInstance {
	if (actor instanceof ActorInstance) {
		return true;
	}

	if (!actor || typeof actor !== "object") {
		return false;
	}

	const candidate = actor as Partial<AnyStaticActorInstance>;
	return (
		typeof candidate.executeAction === "function" &&
		typeof candidate.beginHonoHttpRequest === "function" &&
		typeof candidate.endHonoHttpRequest === "function" &&
		typeof candidate.connectionManager === "object" &&
		candidate.connectionManager !== null
	);
}

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
> implements BaseActorInstance<S, CP, CS, V, I, DB, E, Q>
{
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
	#shutdownComplete = false;
	#sleepTimeout?: NodeJS.Timeout;
	#abortController = new AbortController();

	// MARK: - Variables & Database
	#vars?: V;
	#db?: InferDatabaseClient<DB>;
	#metrics = new ActorMetrics();

	// MARK: - Preload
	#workflowPreloadEntries?: PreloadedEntries;
	#expectNoKvRoundTrips = false;

	// MARK: - Background Tasks
	#backgroundPromises: Promise<void>[] = [];
	#websocketCallbackPromises: Promise<void>[] = [];
	#preventSleepClearedPromise?: ReturnType<typeof promiseWithResolvers<void>>;
	#runPromise?: Promise<void>;
	#runHandlerActive = false;
	#activeQueueWaitCount = 0;

	// MARK: - HTTP/WebSocket Tracking
	#activeHonoHttpRequests = 0;
	#activeAsyncRegionCounts: ActiveAsyncRegionCounts = {
		keepAwake: 0,
		internalKeepAwake: 0,
		websocketCallbacks: 0,
	};
	#preventSleep = false;

	// MARK: - Deprecated (kept for compatibility)
	#schedule!: Schedule;

	// MARK: - Hibernatable WebSocket State
	#hibernatableWebSocketAckState = new HibernatableWebSocketAckState();

	// MARK: - Inspector
	#inspectorToken?: string;
	#inspector: ActorInspector;

	// MARK: - Tracing
	#traces!: Traces<OtlpExportTraceServiceRequestJson>;

	// MARK: - Driver Overrides
	/**
	 * Per-instance config option overrides applied by the driver after creation.
	 * When set, the effective option value is the minimum of the base config
	 * value and the override value.
	 */
	overrides: {
		sleepGracePeriod?: number;
		onSleepTimeout?: number;
		onDestroyTimeout?: number;
		runStopTimeout?: number;
		waitUntilTimeout?: number;
	} = {};

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

	get metrics(): ActorMetrics {
		return this.#metrics;
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

	get workflowPreloadEntries(): PreloadedEntries | undefined {
		return this.#workflowPreloadEntries;
	}

	[WARN_UNEXPECTED_KV_ROUND_TRIP](method: string): void {
		if (this.#expectNoKvRoundTrips) {
			this.#rLog.warn({
				msg: "unexpected KV round-trip during startup",
				method,
			});
			this.#expectNoKvRoundTrips = false;
		}
	}

	static #userStartupKeys: Set<StartupTimingKey> = new Set([
		"createStateMs",
		"onCreateMs",
		"onWakeMs",
		"createVarsMs",
		"dbMigrateMs",
	]);

	/**
	 * Measure the duration of an async startup step. Logs at debug level
	 * and records the duration on the startup metrics object.
	 *
	 * When `pauseKvGuard` is true, the unexpected KV round-trip guard is
	 * suspended for the duration of the callback (used for user code
	 * callbacks that may legitimately issue KV reads).
	 */
	async #measureStartup<T>(
		name: StartupTimingKey,
		fn: () => Promise<T> | T,
		opts?: { pauseKvGuard?: boolean },
	): Promise<T> {
		const savedGuard = this.#expectNoKvRoundTrips;
		if (opts?.pauseKvGuard) {
			this.#expectNoKvRoundTrips = false;
		}
		const start = performance.now();
		try {
			const result = await fn();
			return result;
		} finally {
			const durationMs = performance.now() - start;
			this.#metrics.startup[name] = durationMs;
			const prefix = ActorInstance.#userStartupKeys.has(name)
				? "perf user"
				: "perf internal";
			this.#rLog.debug({ msg: `${prefix}: ${name}`, durationMs });
			if (opts?.pauseKvGuard) {
				this.#expectNoKvRoundTrips = savedGuard;
			}
		}
	}

	get conns(): Map<ConnId, Conn<S, CP, CS, V, I, DB, E, Q>> {
		return this.connectionManager.connections;
	}

	/**
	 * Records delivery of an inbound indexed hibernatable websocket message and
	 * schedules persistence so the index is only acked after a durable write.
	 */
	handleInboundHibernatableWebSocketMessage(
		conn: AnyConn | undefined,
		payload: InputData,
		rivetMessageIndex: number | undefined,
	): void {
		if (!conn?.isHibernatable) {
			return;
		}

		const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
		const hibernatable = connStateManager.hibernatableData;
		if (!hibernatable) {
			return;
		}

		invariant(
			typeof rivetMessageIndex === "number",
			"missing rivetMessageIndex for hibernatable websocket message",
		);

		applyInboundHibernatableWebSocketMessage({
			connId: conn.id,
			hibernatable,
			messageLength: getValueLength(payload),
			rivetMessageIndex,
			ackState: this.#hibernatableWebSocketAckState,
			saveState: (opts) => {
				void this.stateManager.saveState(opts).catch((error) => {
					this.#rLog.error({
						msg: "failed to schedule hibernatable websocket persistence",
						connId: conn.id,
						error: stringifyError(error),
					});
				});
			},
		});
	}

	onCreateHibernatableConn(conn: AnyConn): void {
		const hibernatable = conn[CONN_STATE_MANAGER_SYMBOL].hibernatableData;
		if (!hibernatable) {
			return;
		}

		this.#hibernatableWebSocketAckState.createConnEntry(
			conn.id,
			hibernatable.serverMessageIndex,
		);
	}

	onDestroyHibernatableConn(conn: AnyConn): void {
		this.#hibernatableWebSocketAckState.deleteConnEntry(conn.id);
	}

	onBeforePersistHibernatableConn(conn: AnyConn): void {
		const hibernatable =
			conn[CONN_STATE_MANAGER_SYMBOL].hibernatableDataOrError();
		this.#hibernatableWebSocketAckState.onBeforePersist(
			conn.id,
			hibernatable.serverMessageIndex,
		);
	}

	onAfterPersistHibernatableConn(conn: AnyConn): void {
		const hibernatable =
			conn[CONN_STATE_MANAGER_SYMBOL].hibernatableDataOrError();
		const ackServerMessageIndex =
			this.#hibernatableWebSocketAckState.consumeAck(conn.id);
		if (ackServerMessageIndex === undefined) {
			return;
		}

		this.driver.ackHibernatableWebSocketMessage?.(
			hibernatable.gatewayId,
			hibernatable.requestId,
			ackServerMessageIndex,
		);
	}

	getHibernatingWebSocketMetadata(): Array<{
		gatewayId: ArrayBuffer;
		requestId: ArrayBuffer;
		serverMessageIndex: number;
		clientMessageIndex: number;
		path: string;
		headers: Record<string, string>;
	}> {
		return Array.from(this.conns.values(), (conn) => {
			const hibernatable =
				conn[CONN_STATE_MANAGER_SYMBOL].hibernatableData;
			if (!hibernatable) {
				return undefined;
			}
			return {
				gatewayId: hibernatable.gatewayId.slice(0),
				requestId: hibernatable.requestId.slice(0),
				serverMessageIndex: hibernatable.serverMessageIndex,
				clientMessageIndex: hibernatable.clientMessageIndex,
				path: hibernatable.requestPath,
				headers: { ...hibernatable.requestHeaders },
			};
		}).filter((entry) => entry !== undefined);
	}

	get schedule(): Schedule {
		return this.#schedule;
	}

	get abortSignal(): AbortSignal {
		return this.#abortController.signal;
	}

	get preventSleep(): boolean {
		return this.#preventSleep;
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
			if (this.#shutdownComplete && "db" in this.#config) {
				throw new errors.ActorStopping(
					"database accessed after actor stopped. If you are using setInterval or other background timers, clean them up with c.abortSignal.",
				);
			}
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
		preload?: PreloadMap,
	) {
		const startupStart = performance.now();

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

		// Enable unexpected KV round-trip detection when preload data was
		// provided.
		if (preload) {
			this.#expectNoKvRoundTrips = true;
		}

		// Extract workflow preload data for lazy consumption by workflow engine.
		if (preload) {
			const workflowEntries = preload.listPrefix(workflowStoragePrefix());
			if (workflowEntries !== undefined) {
				this.#workflowPreloadEntries = workflowEntries;
			}
		}

		// Setup database before lifecycle hooks so c.db is available in
		// createState, onCreate, createVars, and onWake.
		await this.#setupDatabase(preload);

		// Create a write collector to batch new-actor init writes into a
		// single kvBatchPut.
		const writeCollector = new WriteCollector(actorDriver, actorId);

		// Load state
		await this.#measureStartup("loadStateMs", () =>
			this.#loadState(preload, writeCollector),
		);

		await this.#measureStartup("initQueueMs", () =>
			this.queueManager.initialize(preload, writeCollector),
		);

		await this.#measureStartup("initInspectorTokenMs", () =>
			this.#initializeInspectorToken(preload, writeCollector),
		);

		// Flush any batched writes from new actor initialization.
		await this.#measureStartup("flushWritesMs", async () => {
			this.#metrics.startup.flushWritesEntries = writeCollector.size;
			await writeCollector.flush();
		});

		// Initialize variables.
		await this.#measureStartup(
			"createVarsMs",
			async () => {
				if (this.#varsEnabled) {
					await this.#initializeVars();
				}
			},
			{ pauseKvGuard: true },
		);

		// Call onStart lifecycle.
		await this.#measureStartup("onWakeMs", () => this.#callOnStart(), {
			pauseKvGuard: true,
		});
		// Initialize alarms
		await this.#measureStartup("initAlarmsMs", () =>
			this.#scheduleManager.initializeAlarms(),
		);

		// Mark as ready
		this.#ready = true;

		// Finish up any remaining initiation
		//
		// Do this after #ready = true since this can call any actor callbacks
		// (which require #assertReady)
		await this.#measureStartup("onBeforeActorStartMs", async () => {
			await this.driver.onBeforeActorStart?.(this);
		});

		// Mark as started
		//
		// We do this after onBeforeActorStart to prevent the actor from going
		// to sleep before finishing setup
		this.#started = true;

		// Clear KV round-trip detection after startup completes.
		this.#expectNoKvRoundTrips = false;

		// Release workflow preload data after startup completes.
		this.#workflowPreloadEntries = undefined;

		// Record total startup time.
		this.#metrics.startup.totalMs = performance.now() - startupStart;
		this.#rLog.info({
			msg: "actor started",
			startupMs: this.#metrics.startup.totalMs,
			kvRoundTrips: this.#metrics.startup.kvRoundTrips,
		});

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

	assertReady() {
		if (!this.#ready) throw new errors.InternalError("Actor not ready");
		this.assertNotShutdown();
	}

	assertNotShutdown() {
		if (this.#shutdownComplete)
			throw new errors.ActorStopping("Actor has shut down");
	}

	async cleanupPersistedConnections(reason?: string): Promise<number> {
		this.assertReady();
		return await this.connectionManager.cleanupPersistedHibernatableConnections(
			reason,
		);
	}

	async restartRunHandler(): Promise<void> {
		this.assertReady();
		if (this.#stopCalled)
			throw new errors.InternalError("Actor is stopping");
		if (this.#runHandlerActive && this.#runPromise) {
			await this.#runPromise;
		}
		if (this.#runHandlerActive) {
			return;
		}

		this.#startRunHandler();
	}

	isRunHandlerActive(): boolean {
		return this.#runHandlerActive;
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

			// Cancel alarm timeouts so they cannot fire during shutdown.
			// Scheduled events are persisted and will be re-initialized
			// on wake via initializeAlarms().
			this.driver.cancelAlarm?.(this.#actorId);

			// Abort listeners in the canonical stop path.
			// This must run for all stop modes, including sleep and remote stop.
			// Destroy may have already triggered an early abort, but repeating abort
			// is intentional and safe.
			try {
				this.#abortController.abort();
			} catch {}

			// Wait for run handler to complete
			await this.#waitForRunHandler(
				this.overrides.runStopTimeout !== undefined
					? Math.min(
							this.#config.options.runStopTimeout,
							this.overrides.runStopTimeout,
						)
					: this.#config.options.runStopTimeout,
			);

			const shutdownTaskDeadlineTs =
				Date.now() + this.#getEffectiveSleepGracePeriod();

			// Call onStop lifecycle
			if (mode === "sleep") {
				await this.#callOnSleep(shutdownTaskDeadlineTs);
			} else if (mode === "destroy") {
				await this.#callOnDestroy();
			} else {
				assertUnreachable(mode);
			}

			// Wait for shutdown tasks that were already in flight before
			// connection teardown starts.
			await this.#waitShutdownTasks(shutdownTaskDeadlineTs);

			// Disconnect non-hibernatable connections
			await this.#disconnectConnections();

			// Drain async WebSocket close handlers and any waitUntil work they
			// enqueue before persisting final state.
			await this.#waitShutdownTasks(shutdownTaskDeadlineTs);

			// Clear timeouts and save state
			this.#rLog.info({ msg: "clearing pending save timeouts" });
			this.stateManager.clearPendingSaveTimeout();
			this.#rLog.info({ msg: "saving state immediately" });
			await this.stateManager.saveState({
				immediate: true,
			});

			// Wait for write queues
			await this.stateManager.waitForPendingWrites();
			await this.#scheduleManager.waitForPendingAlarmWrites();
		} finally {
			this.#shutdownComplete = true;
			await this.#cleanupDatabase();
		}
	}

	async debugForceCrash() {
		if (this.#shutdownComplete) {
			return;
		}
		if (this.#stopCalled) {
			this.#rLog.warn({
				msg: "already stopping actor during hard crash",
			});
			return;
		}
		this.#stopCalled = true;

		try {
			if (this.#sleepTimeout) {
				clearTimeout(this.#sleepTimeout);
				this.#sleepTimeout = undefined;
			}

			this.driver.cancelAlarm?.(this.#actorId);
			this.stateManager.clearPendingSaveTimeout();

			try {
				this.#abortController.abort();
			} catch {}
		} finally {
			this.#shutdownComplete = true;
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

		// Abort immediately so in flight waits can exit before the driver stop
		// handshake completes.
		// The onStop path will call abort again as a safety net for all stop
		// modes.
		try {
			this.#abortController.abort();
		} catch {}

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
		// Hibernating WebSocket connections intentionally do not keep the
		// actor alive so the actor can sleep while connections are idle.
		// Reset the sleep timer on each message so the actor stays awake
		// while clients are actively communicating.
		this.resetSleepTimer();

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
		const canSubscribe = getEventCanSubscribe(
			this.#config.events,
			eventName,
		);
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
	async invokeActionByName(
		ctx: ActorContext<S, CP, CS, V, I, DB, E, Q>,
		actionName: string,
		args: unknown[],
		timeoutMs?: number,
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

		const outputOrPromise = actionFunction.call(
			undefined,
			// TODO: Replace this cast after scheduled actions and direct actions
			// share a properly typed internal action invocation context.
			ctx as any,
			...args,
		);
		const maybeThenable = outputOrPromise as {
			then?: (onfulfilled?: unknown, onrejected?: unknown) => unknown;
		};
		if (maybeThenable && typeof maybeThenable.then === "function") {
			const promise = Promise.resolve(outputOrPromise);
			return await (timeoutMs === undefined
				? promise
				: deadline(promise, timeoutMs));
		}

		return outputOrPromise;
	}

	async executeAction(
		ctx: ActionContext<S, CP, CS, V, I, DB, E, Q>,
		actionName: string,
		args: unknown[],
	): Promise<unknown> {
		this.assertReady();

		this.#beginActiveAsyncRegion("internalKeepAwake");
		this.#metrics.actionCalls++;
		const actionStart = performance.now();
		const actionSpan = this.startTraceSpan(`actor.action.${actionName}`, {
			"rivet.action.name": actionName,
		});
		let spanEnded = false;

		try {
			const output = await this.#traces.withSpan(actionSpan, async () => {
				this.#rLog.debug({
					msg: "executing action",
					actionName,
					args,
				});

				let output = await this.invokeActionByName(
					ctx,
					actionName,
					args,
					this.#config.options.actionTimeout,
				);

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

			return output;
		} catch (error) {
			this.#metrics.actionErrors++;
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
			this.#metrics.actionTotalMs += performance.now() - actionStart;
			if (!spanEnded && actionSpan.isActive()) {
				this.#traces.endSpan(actionSpan, {
					status: { code: "OK" },
				});
			}
			this.#endActiveAsyncRegion("internalKeepAwake");
			this.stateManager.savePersistThrottled();
		}
	}

	// MARK: - HTTP/WebSocket Handlers
	//
	// handleRawRequest intentionally has no isStopping guard (unlike
	// handleRawWebSocket). In-flight HTTP requests from pre-existing
	// connections are allowed during the graceful shutdown window.
	// New external requests cannot reach a stopping actor because the
	// driver layer blocks them.
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
		if (this.#stopCalled)
			throw new errors.InternalError("Actor is stopping");

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
			const trackedWebSocket = this.#createTrackedWebSocket(websocket);

			// NOTE: This is async and will run in the background
			const voidOrPromise = this.#traces.withSpan(span, () =>
				this.#config.onWebSocket!(ctx, trackedWebSocket),
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
		if (this.#stopCalled) return;
		this.resetSleepTimer();
		await this.#scheduleManager.onAlarm();
	}

	// MARK: - Background Tasks
	waitUntil(promise: Promise<void>) {
		this.assertNotShutdown();

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

	#getEffectiveSleepGracePeriod(): number {
		// Resolve the graceful shutdown budget for sleep.
		//
		// If sleepGracePeriod is unset, use the new default unless one of the
		// deprecated legacy timeout knobs was explicitly customized. In that case,
		// keep honoring the legacy sum so existing tuned actors do not silently
		// lose shutdown budget.
		if (this.overrides.sleepGracePeriod !== undefined) {
			return this.#config.options.sleepGracePeriod !== undefined
				? Math.min(
						this.#config.options.sleepGracePeriod,
						this.overrides.sleepGracePeriod,
					)
				: this.overrides.sleepGracePeriod;
		}

		if (this.#config.options.sleepGracePeriod !== undefined) {
			return this.#config.options.sleepGracePeriod;
		}

		const effectiveOnSleepTimeout =
			this.overrides.onSleepTimeout !== undefined
				? Math.min(
						this.#config.options.onSleepTimeout,
						this.overrides.onSleepTimeout,
					)
				: this.#config.options.onSleepTimeout;
		const effectiveWaitUntilTimeout =
			this.overrides.waitUntilTimeout !== undefined
				? Math.min(
						this.#config.options.waitUntilTimeout,
						this.overrides.waitUntilTimeout,
					)
				: this.#config.options.waitUntilTimeout;

		const usesDefaultLegacyTimeouts =
			effectiveOnSleepTimeout === DEFAULT_ON_SLEEP_TIMEOUT &&
			effectiveWaitUntilTimeout === DEFAULT_WAIT_UNTIL_TIMEOUT;
		if (usesDefaultLegacyTimeouts) {
			return DEFAULT_SLEEP_GRACE_PERIOD;
		}

		return effectiveOnSleepTimeout + effectiveWaitUntilTimeout;
	}

	#beginActiveAsyncRegion(region: keyof ActiveAsyncRegionCounts) {
		this.#activeAsyncRegionCounts[region]++;
		this.resetSleepTimer();
	}

	#endActiveAsyncRegion(region: keyof ActiveAsyncRegionCounts) {
		this.#activeAsyncRegionCounts[region]--;
		if (this.#activeAsyncRegionCounts[region] < 0) {
			this.#activeAsyncRegionCounts[region] = 0;
			this.#rLog.warn({
				msg: ACTIVE_ASYNC_REGION_ERROR_MESSAGES[region],
				...EXTRA_ERROR_LOG,
			});
		}

		this.resetSleepTimer();
	}

	#trackWebSocketCallback(eventType: string, promise: Promise<void>) {
		this.#beginActiveAsyncRegion("websocketCallbacks");

		const trackedPromise = promise
			.then(() => {
				this.#rLog.debug({
					msg: "websocket callback complete",
					eventType,
				});
			})
			.catch((error) => {
				this.#rLog.error({
					msg: "websocket callback failed",
					eventType,
					error: stringifyError(error),
				});
			})
			.finally(() => {
				this.#endActiveAsyncRegion("websocketCallbacks");
			});

		this.#websocketCallbackPromises.push(trackedPromise);
	}

	/**
	 * Prevents the actor from sleeping while the given promise is running.
	 *
	 * Use this when performing async operations in the `run` handler or other
	 * background contexts where you need to ensure the actor stays awake.
	 *
	 * Returns the resolved value and resets the sleep timer on completion.
	 * Errors are propagated to the caller.
	 *
	 * @deprecated Use `setPreventSleep(true)` while work is active, or move
	 * shutdown and flush work to `onSleep` if it can wait until the actor is
	 * sleeping.
	 */
	async keepAwake<T>(promise: Promise<T>): Promise<T> {
		this.assertNotShutdown();

		this.#beginActiveAsyncRegion("keepAwake");

		try {
			return await promise;
		} finally {
			this.#endActiveAsyncRegion("keepAwake");
		}
	}

	/**
	 * Internal sleep blocker used by runtime subsystems.
	 *
	 * Accepts either a promise or a thunk. The thunk form exists so the actor
	 * can enter the sleep-blocking region before user code starts running. This
	 * avoids a race where work begins, but the actor is not yet marked active,
	 * which can allow the sleep timer to fire underneath that work.
	 */
	internalKeepAwake<T>(promise: Promise<T>): Promise<T>;
	internalKeepAwake<T>(run: () => T | Promise<T>): Promise<T>;
	async internalKeepAwake<T>(
		promiseOrRun: Promise<T> | (() => T | Promise<T>),
	): Promise<T> {
		this.assertNotShutdown();

		this.#beginActiveAsyncRegion("internalKeepAwake");

		try {
			if (typeof promiseOrRun === "function") {
				return await promiseOrRun();
			}
			return await promiseOrRun;
		} finally {
			this.#endActiveAsyncRegion("internalKeepAwake");
		}
	}

	setPreventSleep(prevent: boolean) {
		if (this.#preventSleep === prevent) return;

		this.#preventSleep = prevent;
		if (!prevent) {
			this.#preventSleepClearedPromise?.resolve();
			this.#preventSleepClearedPromise = undefined;
		}
		this.#rLog.debug({
			msg: "updated prevent sleep state",
			prevent,
		});
		this.resetSleepTimer();
	}

	beginQueueWait() {
		this.assertReady();
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
		if (getRivetExperimentalOtel()) {
			// Experimental mode persists trace data to actor storage so inspector
			// queries can return OTel payloads.
			this.#traces = createTraces({
				driver: new ActorTracesDriver(this.driver, this.#actorId),
			});
		} else {
			// Keep the tracing API calls active while disabling trace persistence
			// until the experimental flag is enabled.
			this.#traces = createNoopTraces();
		}
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

	async #loadState(preload?: PreloadMap, writeCollector?: WriteCollector) {
		let persistDataBuffer: Uint8Array | null;
		const preloaded = preload?.get(KEYS.PERSIST_DATA);
		if (preloaded) {
			persistDataBuffer = preloaded.value;
		} else {
			this[WARN_UNEXPECTED_KV_ROUND_TRIP]("kvBatchGet");
			this.#metrics.startup.kvRoundTrips++;
			const [buf] = await this.driver.kvBatchGet(this.#actorId, [
				KEYS.PERSIST_DATA,
			]);
			persistDataBuffer = buf;
		}

		invariant(
			persistDataBuffer !== null,
			"persist data has not been set, it should be set when initialized",
		);

		const bareData =
			ACTOR_VERSIONED.deserializeWithEmbeddedVersion(persistDataBuffer);
		const persistData = convertActorFromBarePersisted<S, I>(bareData);

		if (persistData.hasInitialized) {
			await this.#measureStartup("restoreConnectionsMs", () =>
				this.#restoreExistingActor(persistData, preload),
			);
		} else {
			this.#metrics.startup.isNew = true;
			await this.#createNewActor(persistData, writeCollector);
		}

		// Pass persist reference to schedule manager
		this.#scheduleManager.setPersist(this.stateManager.persist);
	}

	async #createNewActor(
		persistData: PersistedActor<S, I>,
		writeCollector?: WriteCollector,
	) {
		this.#rLog.info({ msg: "actor creating" });

		// Initialize state
		await this.#measureStartup("createStateMs", () =>
			this.stateManager.initializeState(persistData, writeCollector),
		);

		// Call onCreate lifecycle
		if (this.#config.onCreate) {
			const onCreate = this.#config.onCreate;
			await this.#measureStartup(
				"onCreateMs",
				() =>
					this.runInTraceSpan("actor.onCreate", undefined, () =>
						onCreate(this.actorContext as any, persistData.input!),
					),
				{ pauseKvGuard: true },
			);
		}
	}

	async #restoreExistingActor(
		persistData: PersistedActor<S, I>,
		preload?: PreloadMap,
	) {
		let connEntries: [Uint8Array, Uint8Array][];
		const preloadedConns = preload?.listPrefix(KEYS.CONN_PREFIX);
		if (preloadedConns !== undefined) {
			connEntries = preloadedConns;
		} else {
			this[WARN_UNEXPECTED_KV_ROUND_TRIP]("kvListPrefix");
			this.#metrics.startup.kvRoundTrips++;
			connEntries = await this.driver.kvListPrefix(
				this.#actorId,
				KEYS.CONN_PREFIX,
			);
		}

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

		this.#metrics.startup.restoreConnectionsCount = connections.length;
		this.#rLog.info({
			msg: "actor restoring",
			connections: connections.length,
		});

		// Initialize state
		this.stateManager.initPersistProxy(persistData);

		// Restore connections
		this.connectionManager.restoreConnections(connections);
	}

	async #initializeInspectorToken(
		preload?: PreloadMap,
		writeCollector?: WriteCollector,
	) {
		let tokenBuffer: Uint8Array | null;
		const preloaded = preload?.get(KEYS.INSPECTOR_TOKEN);
		if (preloaded) {
			tokenBuffer = preloaded.value;
		} else {
			this[WARN_UNEXPECTED_KV_ROUND_TRIP]("kvBatchGet");
			this.#metrics.startup.kvRoundTrips++;
			const [buf] = await this.driver.kvBatchGet(this.#actorId, [
				KEYS.INSPECTOR_TOKEN,
			]);
			tokenBuffer = buf;
		}

		if (tokenBuffer !== null) {
			const decoder = new TextDecoder();
			this.#inspectorToken = decoder.decode(tokenBuffer);
			this.#rLog.debug({ msg: "loaded existing inspector token" });
		} else {
			this.#inspectorToken = generateSecureToken();
			const tokenBytes = new TextEncoder().encode(this.#inspectorToken);
			if (writeCollector) {
				writeCollector.add(KEYS.INSPECTOR_TOKEN, tokenBytes);
			} else {
				this.#metrics.startup.kvRoundTrips++;
				await this.driver.kvBatchPut(this.#actorId, [
					[KEYS.INSPECTOR_TOKEN, tokenBytes],
				]);
			}
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

	async #callOnSleep(deadlineTs: number) {
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
							const remaining = deadlineTs - Date.now();
							if (remaining <= 0) {
								throw new DeadlineError();
							}
							await deadline(result, remaining);
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
								this.overrides.onDestroyTimeout !== undefined
									? Math.min(
											this.#config.options
												.onDestroyTimeout,
											this.overrides.onDestroyTimeout,
										)
									: this.#config.options.onDestroyTimeout,
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

		// Do not destroy or immediately sleep the actor when run exits. Finished
		// workflows must stay inspectable when something goes wrong, and callers
		// may still need to invoke actions after the run handler has completed.
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

					if (runSpan.isActive()) {
						this.endTraceSpan(runSpan, { code: "OK" });
					}
					this.#rLog.info({
						msg: "run handler exited",
					});
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

					this.endTraceSpan(runSpan, {
						code: "ERROR",
						message: stringifyError(error),
					});
					this.#rLog.error({
						msg: "run handler threw error",
						error: stringifyError(error),
					});
				})
				.finally(() => {
					this.#runHandlerActive = false;
					this.resetSleepTimer();
				});
		} else if (runSpan.isActive()) {
			this.endTraceSpan(runSpan, { code: "OK" });
			this.#rLog.info({
				msg: "run handler exited",
			});
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

	async #setupDatabase(preload?: PreloadMap) {
		if (!("db" in this.#config) || !this.#config.db) {
			return;
		}

		const dbProvider = this.#config.db;

		let client: InferDatabaseClient<DB> | undefined;
		try {
			client = await this.#measureStartup("setupDatabaseClientMs", () =>
				dbProvider.createClient({
					actorId: this.#actorId,
					overrideRawDatabaseClient: this.driver
						.overrideRawDatabaseClient
						? () =>
								this.driver.overrideRawDatabaseClient!(
									this.#actorId,
								)
						: undefined,
					overrideDrizzleDatabaseClient: this.driver
						.overrideDrizzleDatabaseClient
						? () =>
								this.driver.overrideDrizzleDatabaseClient!(
									this.#actorId,
								)
						: undefined,
					kv: {
						batchPut: (entries: [Uint8Array, Uint8Array][]) =>
							this.driver.kvBatchPut(this.#actorId, entries),
						batchGet: (keys: Uint8Array[]) =>
							this.driver.kvBatchGet(this.#actorId, keys),
						batchDelete: (keys: Uint8Array[]) =>
							this.driver.kvBatchDelete(this.#actorId, keys),
						deleteRange: (start: Uint8Array, end: Uint8Array) =>
							this.driver.kvDeleteRange(
								this.#actorId,
								start,
								end,
							),
					},
					metrics: this.#metrics,
					log: this.#rLog,
					nativeDatabaseProvider:
						this.driver.getNativeDatabaseProvider?.(),
				}),
			);
			this.#rLog.info({ msg: "database migration starting" });
			await this.#measureStartup("dbMigrateMs", async () => {
				await dbProvider.onMigrate?.(client!);
			});
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
			if (error instanceof Error) {
				this.#rLog.error({
					msg: "database setup failed",
					error: stringifyError(error),
				});
				throw error;
			}
			const wrappedError = new Error(
				`Database setup failed: ${String(error)}`,
			);
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
		const dbConfig = "db" in this.#config ? this.#config.db : undefined;
		this.#db = undefined;

		if (client && dbConfig) {
			try {
				await dbConfig.onDestroy?.(client);
			} catch (error) {
				this.#rLog.error({
					msg: "database cleanup failed",
					error: stringifyError(error),
				});
			}
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
		let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
		const res = await Promise.race([
			Promise.all(promises).then(() => {
				if (timeoutHandle !== undefined) clearTimeout(timeoutHandle);
				return false;
			}),
			new Promise<boolean>((res) => {
				timeoutHandle = globalThis.setTimeout(() => res(true), 1500);
			}),
		]);

		if (res) {
			this.#rLog.warn({
				msg: "timed out waiting for connections to close, shutting down anyway",
			});
		}
	}

	/**
	 * Drain shutdown blockers within the shared shutdown deadline.
	 *
	 * This method is intentionally called multiple times during shutdown so
	 * work created by earlier shutdown phases, such as async WebSocket close
	 * handlers or waitUntil calls they enqueue, is also drained before final
	 * persistence.
	 */
	async #waitShutdownTasks(deadlineTs: number) {
		while (
			this.#backgroundPromises.length > 0 ||
			this.#websocketCallbackPromises.length > 0 ||
			this.#preventSleep
		) {
			await this.#drainPromiseQueue(
				this.#backgroundPromises,
				"background tasks",
				deadlineTs,
			);
			await this.#drainPromiseQueue(
				this.#websocketCallbackPromises,
				"websocket callbacks",
				deadlineTs,
			);
			await this.#waitForPreventSleepClear(deadlineTs);

			if (deadlineTs - Date.now() <= 0) {
				break;
			}
		}
	}

	async #drainPromiseQueue(
		promises: Promise<void>[],
		label: string,
		deadlineTs: number,
	) {
		// Drain in a loop so that work scheduled from earlier callbacks is also
		// awaited within the same deadline.
		while (promises.length > 0) {
			const remaining = deadlineTs - Date.now();
			if (remaining <= 0) {
				this.#rLog.error({
					msg: `timed out waiting for ${label}`,
					count: promises.length,
				});
				break;
			}

			const batch = promises.length;

			let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
			const timedOut = await Promise.race([
				Promise.allSettled(promises.slice(0, batch)).then(() => {
					if (timeoutHandle !== undefined)
						clearTimeout(timeoutHandle);
					return false;
				}),
				new Promise<true>((resolve) => {
					timeoutHandle = setTimeout(() => resolve(true), remaining);
				}),
			]);

			if (timedOut) {
				this.#rLog.error({
					msg: `timed out waiting for ${label}`,
					count: promises.length,
				});
				break;
			}

			promises.splice(0, batch);
		}

		if (promises.length === 0) {
			this.#rLog.debug({ msg: `${label} finished` });
		}
	}

	async #waitForPreventSleepClear(deadlineTs: number) {
		while (this.#preventSleep) {
			const remaining = deadlineTs - Date.now();
			if (remaining <= 0) {
				this.#rLog.error({
					msg: "timed out waiting for preventSleep to clear during shutdown",
				});
				break;
			}

			if (!this.#preventSleepClearedPromise) {
				this.#preventSleepClearedPromise = promiseWithResolvers<void>(
					(reason: unknown) =>
						this.#rLog.warn({
							msg: "preventSleep clear waiter rejected unexpectedly",
							reason: stringifyError(reason),
							...EXTRA_ERROR_LOG,
						}),
				);
			}

			let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
			const timedOut = await Promise.race([
				this.#preventSleepClearedPromise.promise.then(() => {
					if (timeoutHandle !== undefined)
						clearTimeout(timeoutHandle);
					return false;
				}),
				new Promise<true>((resolve) => {
					timeoutHandle = setTimeout(() => resolve(true), remaining);
				}),
			]);

			if (timedOut) {
				this.#rLog.error({
					msg: "timed out waiting for preventSleep to clear during shutdown",
				});
				break;
			}
		}
	}

	#createTrackedWebSocket(websocket: UniversalWebSocket): TrackedWebSocket {
		return new TrackedWebSocket(websocket, {
			onPromise: (eventType, promise) => {
				this.#trackWebSocketCallback(eventType, promise);
			},
			onError: (eventType, error) => {
				this.#rLog.error({
					msg: "error in websocket event handler",
					eventType,
					error: stringifyError(error),
				});
			},
		});
	}

	resetSleepTimer() {
		if (this.#config.options.noSleep || !this.#sleepingSupported) return;
		if (this.#stopCalled) return;

		const canSleep = this.#canSleep();
		let timeoutMs: number | undefined;

		if (canSleep === CanSleep.Yes) {
			timeoutMs = this.#config.options.sleepTimeout;
		}

		this.#rLog.debug({
			msg: "resetting sleep timer",
			canSleep: CanSleep[canSleep],
			existingTimeout: !!this.#sleepTimeout,
			timeout: timeoutMs,
		});

		if (this.#sleepTimeout) {
			clearTimeout(this.#sleepTimeout);
			this.#sleepTimeout = undefined;
		}

		if (this.#sleepCalled) return;

		if (timeoutMs !== undefined) {
			this.#sleepTimeout = setTimeout(() => {
				if (this.#canSleep() !== CanSleep.Yes) {
					this.resetSleepTimer();
					return;
				}
				this.startSleep();
			}, timeoutMs);
		}
	}

	#canSleep(): CanSleep {
		if (!this.#ready) return CanSleep.NotReady;
		if (!this.#started) return CanSleep.NotReady;
		if (this.#preventSleep) return CanSleep.PreventSleep;
		if (this.#activeHonoHttpRequests > 0)
			return CanSleep.ActiveHonoHttpRequests;
		if (this.#activeAsyncRegionCounts.keepAwake > 0) {
			return CanSleep.ActiveKeepAwake;
		}
		if (this.#activeAsyncRegionCounts.internalKeepAwake > 0) {
			return CanSleep.ActiveInternalKeepAwake;
		}
		if (this.#runHandlerActive && this.#activeQueueWaitCount === 0) {
			return CanSleep.ActiveRun;
		}

		for (const _conn of this.connectionManager.connections.values()) {
			if (!_conn.isHibernatable) {
				return CanSleep.ActiveConns;
			}
		}

		if (this.connectionManager.pendingDisconnectCount > 0) {
			return CanSleep.ActiveDisconnectCallbacks;
		}

		if (this.#activeAsyncRegionCounts.websocketCallbacks > 0) {
			return CanSleep.ActiveWebSocketCallbacks;
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
