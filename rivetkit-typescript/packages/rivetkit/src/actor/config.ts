import { z } from "zod/v4";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { Conn } from "./conn/mod";
import type {
	ActionContext,
	ActorContext,
	BeforeActionResponseContext,
	BeforeConnectContext,
	ConnectContext,
	CreateConnStateContext,
	CreateContext,
	CreateVarsContext,
	DestroyContext,
	DisconnectContext,
	RequestContext,
	RunContext,
	SleepContext,
	StateChangeContext,
	WakeContext,
	WebSocketContext,
} from "./contexts";
import type { AnyDatabaseProvider } from "./database";
import type { EventSchemaConfig, QueueSchemaConfig } from "./schema";

export interface ActorTypes<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> {
	state?: TState;
	connParams?: TConnParams;
	connState?: TConnState;
	vars?: TVars;
	input?: TInput;
	database?: TDatabase;
}

// Helper for validating function types - accepts generic for specific function signatures
const zFunction = <
	T extends (...args: any[]) => any = (...args: unknown[]) => unknown,
>() => z.custom<T>((val) => typeof val === "function");

export type InspectorUnsubscribe = () => void;

export interface WorkflowInspectorConfig<THistory = unknown> {
	getHistory: () => THistory | null;
	onHistoryUpdated?: (
		listener: (history: THistory) => void,
	) => InspectorUnsubscribe;
}

export interface RunInspectorConfig<THistory = unknown> {
	workflow?: WorkflowInspectorConfig<THistory>;
}

const WorkflowInspectorConfigSchema = z.object({
	getHistory: zFunction<WorkflowInspectorConfig<unknown>["getHistory"]>(),
	onHistoryUpdated:
		zFunction<
			NonNullable<WorkflowInspectorConfig<unknown>["onHistoryUpdated"]>
		>().optional(),
});

const RunInspectorConfigSchema = z
	.object({
		workflow: WorkflowInspectorConfigSchema.optional(),
	})
	.optional();

// Schema for run handler with metadata
export const RunConfigSchema = z.object({
	/** Display name for the actor in the Inspector UI. */
	name: z.string().optional(),
	/** Icon for the actor in the Inspector UI. Can be an emoji or FontAwesome icon name. */
	icon: z.string().optional(),
	/** The run handler function. */
	run: zFunction(),
	/** Inspector integration for long-running run handlers. */
	inspector: RunInspectorConfigSchema.optional(),
});
type RunConfigRuntime = z.infer<typeof RunConfigSchema>;
export type RunConfig<
	TState = unknown,
	TConnParams = unknown,
	TConnState = unknown,
	TVars = unknown,
	TInput = unknown,
	TDatabase extends AnyDatabaseProvider = AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> = Omit<RunConfigRuntime, "run"> & {
	run: (
		c: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;
};

type AnyRunConfig = RunConfig<
	any,
	any,
	any,
	any,
	any,
	AnyDatabaseProvider,
	any,
	any
>;

export const RUN_FUNCTION_CONFIG_SYMBOL = Symbol.for(
	"rivetkit.run_function_config",
);

interface RunFunctionConfig {
	name?: string;
	icon?: string;
	inspector?: RunInspectorConfig;
}

type RunFunctionWithConfig = ((...args: any[]) => any) & {
	[RUN_FUNCTION_CONFIG_SYMBOL]?: RunFunctionConfig;
};

// Run can be either a function or an object with name/icon/run
const zRunHandler = z.union([zFunction(), RunConfigSchema]).optional();

/** Extract the run function from either a function or RunConfig object. */
export function getRunFunction(
	run: ((...args: any[]) => any) | AnyRunConfig | undefined,
): ((...args: any[]) => any) | undefined {
	if (!run) return undefined;
	if (typeof run === "function") return run;
	return run.run;
}

/** Extract run metadata (name/icon) from RunConfig if provided. */
export function getRunMetadata(
	run: ((...args: any[]) => any) | AnyRunConfig | undefined,
): { name?: string; icon?: string } {
	if (!run) return {};
	if (typeof run === "function") {
		const config = (run as RunFunctionWithConfig)[
			RUN_FUNCTION_CONFIG_SYMBOL
		];
		if (!config) return {};
		return { name: config.name, icon: config.icon };
	}
	return { name: run.name, icon: run.icon };
}

/** Extract run inspector configuration if provided. */
export function getRunInspectorConfig(
	run: ((...args: any[]) => any) | AnyRunConfig | undefined,
): RunInspectorConfig | undefined {
	if (!run) return undefined;
	if (typeof run === "function") {
		return (run as RunFunctionWithConfig)[RUN_FUNCTION_CONFIG_SYMBOL]
			?.inspector;
	}
	return run.inspector;
}

// This schema is used to validate the input at runtime. The generic types are defined below in `ActorConfig`.
//
// We don't use Zod generics with `z.custom` because:
// (a) there seems to be a weird bug in either Zod, tsup, or TSC that causese external packages to have different types from `z.infer` than from within the same package and
// (b) it makes the type definitions incredibly difficult to read as opposed to vanilla TypeScript.
export const ActorConfigSchema = z
	.object({
		onCreate: zFunction().optional(),
		onDestroy: zFunction().optional(),
		onWake: zFunction().optional(),
		onSleep: zFunction().optional(),
		run: zRunHandler,
		onStateChange: zFunction().optional(),
		onBeforeConnect: zFunction().optional(),
		onConnect: zFunction().optional(),
		onDisconnect: zFunction().optional(),
		onBeforeActionResponse: zFunction().optional(),
		onRequest: zFunction().optional(),
		onWebSocket: zFunction().optional(),
		actions: z.record(z.string(), zFunction()).default(() => ({})),
		events: z.record(z.string(), z.any()).optional(),
		queues: z.record(z.string(), z.any()).optional(),
		state: z.any().optional(),
		createState: zFunction().optional(),
		connState: z.any().optional(),
		createConnState: zFunction().optional(),
		vars: z.any().optional(),
		db: z.any().optional(),
		createVars: zFunction().optional(),
		options: z
			.object({
				/** Display name for the actor in the Inspector UI. */
				name: z.string().optional(),
				/** Icon for the actor in the Inspector UI. Can be an emoji or FontAwesome icon name. */
				icon: z.string().optional(),
				createVarsTimeout: z.number().positive().default(5000),
				createConnStateTimeout: z.number().positive().default(5000),
				onConnectTimeout: z.number().positive().default(5000),
				// This must be less than engine config > pegboard.actor_stop_threshold
				onSleepTimeout: z.number().positive().default(5000),
				// This must be less than engine config > pegboard.actor_stop_threshold
				onDestroyTimeout: z.number().positive().default(5000),
				stateSaveInterval: z.number().positive().default(10_000),
				actionTimeout: z.number().positive().default(60_000),
				// Max time to wait for waitUntil background promises during shutdown
				waitUntilTimeout: z.number().positive().default(15_000),
				// Max time to wait for run handler to stop during shutdown
				runStopTimeout: z.number().positive().default(15_000),
				connectionLivenessTimeout: z.number().positive().default(2500),
				connectionLivenessInterval: z.number().positive().default(5000),
				noSleep: z.boolean().default(false),
				sleepTimeout: z.number().positive().default(30_000),
				maxQueueSize: z.number().positive().default(1000),
				maxQueueMessageSize: z
					.number()
					.positive()
					.default(64 * 1024),
				/**
				 * Can hibernate WebSockets for onWebSocket.
				 *
				 * WebSockets using actions/events are hibernatable by default.
				 *
				 * @experimental
				 **/
				canHibernateWebSocket: z
					.union([
						z.boolean(),
						zFunction<(request: Request) => boolean>(),
					])
					.default(false),
			})
			.strict()
			.prefault(() => ({})),
	})
	.strict()
	.refine(
		(data) => !(data.state !== undefined && data.createState !== undefined),
		{
			message: "Cannot define both 'state' and 'createState'",
			path: ["state"],
		},
	)
	.refine(
		(data) =>
			!(
				data.connState !== undefined &&
				data.createConnState !== undefined
			),
		{
			message: "Cannot define both 'connState' and 'createConnState'",
			path: ["connState"],
		},
	)
	.refine(
		(data) => !(data.vars !== undefined && data.createVars !== undefined),
		{
			message: "Cannot define both 'vars' and 'createVars'",
			path: ["vars"],
		},
	);

// Creates state config
//
// This must have only one or the other or else TState will not be able to be inferred
//
// Data returned from this handler will be available on `c.state`.
type CreateState<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> =
	| { state: TState }
	| {
		createState: (
			c: CreateContext<TState, TInput, TDatabase, TEvents, TQueues>,
			input: TInput,
		) => TState | Promise<TState>;
	}
	| Record<never, never>;

// Creates connection state config
//
// This must have only one or the other or else TState will not be able to be inferred
//
// Data returned from this handler will be available on `c.conn.state`.
type CreateConnState<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> =
	| { connState: TConnState }
	| {
		createConnState: (
			c: CreateConnStateContext<
				TState,
				TVars,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
			params: TConnParams,
		) => TConnState | Promise<TConnState>;
	}
	| Record<never, never>;

// Creates vars config
//
// This must have only one or the other or else TState will not be able to be inferred
/**
 * @experimental
 */
type CreateVars<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
> =
	| {
		/**
		 * @experimental
		 */
		vars: TVars;
	}
	| {
		/**
		 * @experimental
		 */
		createVars: (
			c: CreateVarsContext<
				TState,
				TInput,
				TDatabase,
				TEvents,
				TQueues
			>,
			driverCtx: any,
		) => TVars | Promise<TVars>;
	}
	| Record<never, never>;

export interface Actions<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> {
	[Action: string]: (
		c: ActionContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		...args: any[]
	) => any;
}

//export type ActorConfig<TState, TConnParams, TConnState, TVars, TInput, TAuthData> = BaseActorConfig<TState, TConnParams, TConnState, TVars, TInput, TAuthData> &
//	ActorConfigLifecycle<TState, TConnParams, TConnState, TVars, TInput, TAuthData> &
//	CreateState<TState, TConnParams, TConnState, TVars, TInput, TAuthData> &
//	CreateConnState<TState, TConnParams, TConnState, TVars, TInput, TAuthData>;

/**
 * @experimental
 */
export type AuthIntent = "get" | "create" | "connect" | "action" | "message";

interface BaseActorConfig<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
> {
	/**
	 * Called when the actor is first initialized.
	 *
	 * Use this hook to initialize your actor's state.
	 * This is called before any other lifecycle hooks.
	 */
	onCreate?: (
		c: CreateContext<TState, TInput, TDatabase, TEvents, TQueues>,
		input: TInput,
	) => void | Promise<void>;

	/**
	 * Called when the actor is destroyed.
	 */
	onDestroy?: (
		c: DestroyContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;

	/**
	 * Called when the actor is started and ready to receive connections and action.
	 *
	 * Use this hook to initialize resources needed for the actor's operation
	 * (timers, external connections, etc.)
	 *
	 * @returns Void or a Promise that resolves when startup is complete
	 */
	onWake?: (
		c: WakeContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;

	/**
	 * Called when the actor is stopping or sleeping.
	 *
	 * Use this hook to clean up resources, save state, or perform
	 * any shutdown operations before the actor sleeps or stops.
	 *
	 * Not supported on all platforms.
	 *
	 * @returns Void or a Promise that resolves when shutdown is complete
	 */
	onSleep?: (
		c: SleepContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;

	/**
	 * Called after the actor starts up. Does not block actor startup.
	 *
	 * Use this for background tasks like:
	 * - Reading from queues in a loop
	 * - Tick loops for periodic work
	 * - Custom workflow logic
	 *
	 * **Important:** The actor may go to sleep at any time during the `run`
	 * handler. Use `c.keepAwake(promise)` to wrap async operations that should
	 * not be interrupted by sleep.
	 *
	 * The handler receives an abort signal via `c.abortSignal` and a
	 * `c.aborted` alias for loop checks. Use these to gracefully exit.
	 *
	 * If this handler exits or throws, the actor will crash and reschedule.
	 * On shutdown, the actor waits for this handler to complete with a
	 * configurable timeout (options.runStopTimeout, default 15s).
	 *
	 * Can be either a function or a RunConfig object with optional name/icon metadata.
	 *
	 * @returns Void or a Promise. If the promise exits, the actor crashes.
	 */
	run?:
	| ((
		c: RunContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>)
	| RunConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;

	/**
	 * Called when the actor's state changes.
	 *
	 * Use this hook to react to state changes, such as updating
	 * external systems or triggering events.
	 *
	 * State changes made within this hook will NOT trigger
	 * another onStateChange call, preventing infinite recursion.
	 *
	 * @param newState The updated state
	 */
	onStateChange?: (
		c: StateChangeContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		newState: TState,
	) => void;

	/**
	 * Called before a client connects to the actor.
	 *
	 * Use this hook to determine if a connection should be accepted
	 * and to initialize connection-specific state.
	 *
	 * @param opts Connection parameters including client-provided data
	 * @returns The initial connection state or a Promise that resolves to it
	 * @throws Throw an error to reject the connection
	 */
	onBeforeConnect?: (
		c: BeforeConnectContext<
			TState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		params: TConnParams,
	) => void | Promise<void>;

	/**
	 * Called when a client successfully connects to the actor.
	 *
	 * Use this hook to perform actions when a connection is established,
	 * such as sending initial data or updating the actor's state.
	 *
	 * @param conn The connection object
	 * @returns Void or a Promise that resolves when connection handling is complete
	 */
	onConnect?: (
		c: ConnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;

	/**
	 * Called when a client disconnects from the actor.
	 *
	 * Use this hook to clean up resources associated with the connection
	 * or update the actor's state.
	 *
	 * @param conn The connection that is being closed
	 * @returns Void or a Promise that resolves when disconnect handling is complete
	 */
	onDisconnect?: (
		c: DisconnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		conn: Conn<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
	) => void | Promise<void>;

	/**
	 * Called before sending an action response to the client.
	 *
	 * Use this hook to modify or transform the output of an action before it's sent
	 * to the client. This is useful for formatting responses, adding metadata,
	 * or applying transformations to the output.
	 *
	 * @param name The name of the action that was called
	 * @param args The arguments that were passed to the action
	 * @param output The output that will be sent to the client
	 * @returns The modified output to send to the client
	 */
	onBeforeActionResponse?: <Out>(
		c: BeforeActionResponseContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		name: string,
		args: unknown[],
		output: Out,
	) => Out | Promise<Out>;

	/**
	 * Called when a raw HTTP request is made to the actor.
	 *
	 * This handler receives raw HTTP requests made to `/actors/{actorName}/http/*` endpoints.
	 * Use this hook to handle custom HTTP patterns, REST APIs, or other HTTP-based protocols.
	 *
	 * @param c The request context with access to the connection
	 * @param request The raw HTTP request object
	 * @param opts Additional options
	 * @returns A Response object to send back, or void to continue with default routing
	 */
	onRequest?: (
		c: RequestContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		request: Request,
	) => Response | Promise<Response>;

	/**
	 * Called when a raw WebSocket connection is established to the actor.
	 *
	 * This handler receives WebSocket connections made to `/actors/{actorName}/websocket/*` endpoints.
	 * Use this hook to handle custom WebSocket protocols, binary streams, or other WebSocket-based communication.
	 *
	 * @param c The WebSocket context with access to the connection
	 * @param websocket The raw WebSocket connection
	 * @param opts Additional options including the original HTTP upgrade request
	 */
	onWebSocket?: (
		c: WebSocketContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>,
		websocket: UniversalWebSocket,
	) => void | Promise<void>;

	actions?: TActions;

	/**
	 * Schema map for events broadcasted by this actor.
	 */
	events?: TEvents;

	/**
	 * Schema map for queue payloads sent by this actor.
	 */
	queues?: TQueues;
}

type ActorDatabaseConfig<TDatabase extends AnyDatabaseProvider> =
	| {
		/**
		 * @experimental
		 */
		db: TDatabase;
	}
	| Record<never, never>;

// 1. Infer schema
// 2. Omit keys that we'll manually define (because of generics)
// 3. Define our own types that have generic constraints
export type ActorConfig<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig = Record<never, never>,
	TQueues extends QueueSchemaConfig = Record<never, never>,
> = Omit<
	z.infer<typeof ActorConfigSchema>,
	| "actions"
	| "events"
	| "queues"
	| "onCreate"
	| "onDestroy"
	| "onWake"
	| "onSleep"
	| "run"
	| "onStateChange"
	| "onBeforeConnect"
	| "onConnect"
	| "onDisconnect"
	| "onBeforeActionResponse"
	| "onRequest"
	| "onWebSocket"
	| "state"
	| "createState"
	| "connState"
	| "createConnState"
	| "vars"
	| "createVars"
	| "db"
> &
	BaseActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues,
		Actions<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase,
			TEvents,
			TQueues
		>
	> &
	CreateState<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	CreateConnState<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	CreateVars<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	ActorDatabaseConfig<TDatabase>;

// See description on `ActorConfig`
export type ActorConfigInput<
	TState = undefined,
	TConnParams = undefined,
	TConnState = undefined,
	TVars = undefined,
	TInput = undefined,
	TDatabase extends AnyDatabaseProvider = undefined,
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
	> = Record<never, never>,
> = {
	types?: ActorTypes<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>;
} & Omit<
	z.input<typeof ActorConfigSchema>,
	| "actions"
	| "events"
	| "queues"
	| "onCreate"
	| "onDestroy"
	| "onWake"
	| "onSleep"
	| "run"
	| "onStateChange"
	| "onBeforeConnect"
	| "onConnect"
	| "onDisconnect"
	| "onBeforeActionResponse"
	| "onRequest"
	| "onWebSocket"
	| "state"
	| "createState"
	| "connState"
	| "createConnState"
	| "vars"
	| "createVars"
	| "db"
> &
	BaseActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues,
		TActions
	> &
	CreateState<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	CreateConnState<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	CreateVars<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> &
	ActorDatabaseConfig<TDatabase>;

// For testing type definitions:
export function test<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends EventSchemaConfig,
	TQueues extends QueueSchemaConfig,
	TActions extends Actions<
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
): ActorConfig<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues
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
	return config;
}

// MARK: Documentation Schema
// This schema is JSON-serializable for documentation generation.
// It excludes function types and focuses on the configurable options.

export const DocActorOptionsSchema = z
	.object({
		name: z
			.string()
			.optional()
			.describe("Display name for the actor in the Inspector UI."),
		icon: z
			.string()
			.optional()
			.describe(
				"Icon for the actor in the Inspector UI. Can be an emoji (e.g., '🚀') or FontAwesome icon name (e.g., 'rocket').",
			),
		createVarsTimeout: z
			.number()
			.optional()
			.describe("Timeout in ms for createVars handler. Default: 5000"),
		createConnStateTimeout: z
			.number()
			.optional()
			.describe(
				"Timeout in ms for createConnState handler. Default: 5000",
			),
		onConnectTimeout: z
			.number()
			.optional()
			.describe("Timeout in ms for onConnect handler. Default: 5000"),
		onSleepTimeout: z
			.number()
			.optional()
			.describe(
				"Timeout in ms for onSleep handler. Must be less than ACTOR_STOP_THRESHOLD_MS. Default: 5000",
			),
		onDestroyTimeout: z
			.number()
			.optional()
			.describe("Timeout in ms for onDestroy handler. Default: 5000"),
		stateSaveInterval: z
			.number()
			.optional()
			.describe(
				"Interval in ms between automatic state saves. Default: 10000",
			),
		actionTimeout: z
			.number()
			.optional()
			.describe("Timeout in ms for action handlers. Default: 60000"),
		waitUntilTimeout: z
			.number()
			.optional()
			.describe(
				"Max time in ms to wait for waitUntil background promises during shutdown. Default: 15000",
			),
		runStopTimeout: z
			.number()
			.optional()
			.describe(
				"Max time in ms to wait for run handler to stop during shutdown. Default: 15000",
			),
		connectionLivenessTimeout: z
			.number()
			.optional()
			.describe(
				"Timeout in ms for connection liveness checks. Default: 2500",
			),
		connectionLivenessInterval: z
			.number()
			.optional()
			.describe(
				"Interval in ms between connection liveness checks. Default: 5000",
			),
		noSleep: z
			.boolean()
			.optional()
			.describe("If true, the actor will never sleep. Default: false"),
		sleepTimeout: z
			.number()
			.optional()
			.describe(
				"Time in ms of inactivity before the actor sleeps. Default: 30000",
			),
		maxQueueSize: z
			.number()
			.optional()
			.describe(
				"Maximum number of queue messages before rejecting new messages. Default: 1000",
			),
		maxQueueMessageSize: z
			.number()
			.optional()
			.describe(
				"Maximum size of each queue message in bytes. Default: 65536",
			),
		canHibernateWebSocket: z
			.boolean()
			.optional()
			.describe(
				"Whether WebSockets using onWebSocket can be hibernated. WebSockets using actions/events are hibernatable by default. Default: false",
			),
	})
	.describe("Actor options for timeouts and behavior configuration.");

export const DocActorConfigSchema = z
	.object({
		state: z
			.unknown()
			.optional()
			.describe(
				"Initial state value for the actor. Cannot be used with createState.",
			),
		createState: z
			.unknown()
			.optional()
			.describe(
				"Function to create initial state. Receives context and input. Cannot be used with state.",
			),
		connState: z
			.unknown()
			.optional()
			.describe(
				"Initial connection state value. Cannot be used with createConnState.",
			),
		createConnState: z
			.unknown()
			.optional()
			.describe(
				"Function to create connection state. Receives context and connection params. Cannot be used with connState.",
			),
		vars: z
			.unknown()
			.optional()
			.describe(
				"Initial ephemeral variables value. Cannot be used with createVars.",
			),
		createVars: z
			.unknown()
			.optional()
			.describe(
				"Function to create ephemeral variables. Receives context and driver context. Cannot be used with vars.",
			),
		db: z
			.unknown()
			.optional()
			.describe("Database provider instance for the actor."),
		onCreate: z
			.unknown()
			.optional()
			.describe(
				"Called when the actor is first initialized. Use to initialize state.",
			),
		onDestroy: z
			.unknown()
			.optional()
			.describe("Called when the actor is destroyed."),
		onWake: z
			.unknown()
			.optional()
			.describe(
				"Called when the actor wakes up and is ready to receive connections and actions.",
			),
		onSleep: z
			.unknown()
			.optional()
			.describe(
				"Called when the actor is stopping or sleeping. Use to clean up resources.",
			),
		run: z
			.unknown()
			.optional()
			.describe(
				"Called after actor starts. Does not block startup. Use for background tasks like queue processing or tick loops. If it exits or throws, the actor crashes.",
			),
			onStateChange: z
				.unknown()
				.optional()
				.describe(
					"Called when the actor's state changes. State changes within this hook won't trigger recursion.",
				),
			onBeforeConnect: z
				.unknown()
				.optional()
				.describe(
					"Called before a client connects. Throw an error to reject the connection.",
				),
			onConnect: z
				.unknown()
				.optional()
				.describe("Called when a client successfully connects."),
		onDisconnect: z
			.unknown()
			.optional()
			.describe("Called when a client disconnects."),
		onBeforeActionResponse: z
			.unknown()
			.optional()
			.describe(
				"Called before sending an action response. Use to transform output.",
			),
		onRequest: z
			.unknown()
			.optional()
			.describe(
				"Called for raw HTTP requests to /actors/{name}/http/* endpoints.",
			),
		onWebSocket: z
			.unknown()
			.optional()
			.describe(
				"Called for raw WebSocket connections to /actors/{name}/websocket/* endpoints.",
			),
		actions: z
			.record(z.string(), z.unknown())
			.optional()
			.describe(
				"Map of action name to handler function. Defaults to an empty object.",
			),
		events: z
			.record(z.string(), z.unknown())
			.optional()
			.describe("Map of event names to schemas."),
		queues: z
			.record(z.string(), z.unknown())
			.optional()
			.describe("Map of queue names to schemas."),
		options: DocActorOptionsSchema.optional(),
	})
	.describe("Actor configuration passed to the actor() function.");
