import { z } from "zod";
import type { UniversalWebSocket } from "@/common/websocket-interface";
import type { Conn } from "./conn/mod";
import type { ActionContext } from "./contexts/action";
import type { ActorContext } from "./contexts/actor";
import type { CreateConnStateContext } from "./contexts/create-conn-state";
import type { OnBeforeConnectContext } from "./contexts/on-before-connect";
import type { OnConnectContext } from "./contexts/on-connect";
import type { RequestContext } from "./contexts/request";
import type { WebSocketContext } from "./contexts/websocket";
import type { AnyDatabaseProvider } from "./database";

export type InitContext = ActorContext<
	undefined,
	undefined,
	undefined,
	undefined,
	undefined,
	undefined
>;

// MARK: - Concurrency Types

/**
 * Concurrency mode for actions and hooks.
 *
 * - `serial`: Waits for all serial & parallel operations to complete before executing.
 *             Blocks other serial & parallel operations from running while executing.
 * - `parallel`: Waits for all serial operations to complete before executing.
 *               Can run concurrently with other parallel operations.
 * - `readonly`: Does not wait for or block any other operations. Can always run immediately.
 *
 * @default "serial"
 */
export type Concurrency = "serial" | "parallel" | "readonly";

/**
 * Base options for concurrency control.
 */
export interface ConcurrencyOptions {
	/**
	 * Concurrency mode for this operation.
	 *
	 * - `serial`: Waits for all serial & parallel operations to complete before executing.
	 *             Blocks other serial & parallel operations from running while executing.
	 * - `parallel`: Waits for all serial operations to complete before executing.
	 *               Can run concurrently with other parallel operations.
	 * - `readonly`: Does not wait for or block any other operations. Can always run immediately.
	 *
	 * @default "serial"
	 */
	concurrency?: Concurrency;
}

/**
 * Configuration options for an individual action.
 */
export interface ActionOptions extends ConcurrencyOptions {
	/**
	 * Custom timeout for this action in milliseconds.
	 * Overrides the default actionTimeout from actor options.
	 */
	timeout?: number;
}

/**
 * Configuration options for a hook.
 */
export type HookOptions = ConcurrencyOptions;

// MARK: - Wrapped Definition Types

/**
 * Symbol used to identify wrapped definitions (actions and hooks).
 */
export const WRAPPED_DEFINITION_SYMBOL = Symbol.for("rivet.wrappedDefinition");

/**
 * A wrapped definition containing the handler and options.
 * Used for both actions and hooks.
 */
export interface WrappedDefinition<
	THandler extends (...args: any[]) => any,
	TOptions,
> {
	[WRAPPED_DEFINITION_SYMBOL]: true;
	handler: THandler;
	options: TOptions;
}

/**
 * Type guard to check if a value is a WrappedDefinition.
 */
export function isWrappedDefinition<
	THandler extends (...args: any[]) => any,
	TOptions,
>(
	value: THandler | WrappedDefinition<THandler, TOptions> | undefined,
): value is WrappedDefinition<THandler, TOptions> {
	return (
		typeof value === "object" &&
		value !== null &&
		WRAPPED_DEFINITION_SYMBOL in value
	);
}

/**
 * A wrapped action definition containing the handler and options.
 */
export type ActionDefinition<THandler extends (...args: any[]) => any> =
	WrappedDefinition<THandler, ActionOptions>;

/**
 * A wrapped hook definition containing the handler and options.
 */
export type HookDefinition<THandler extends (...args: any[]) => any> =
	WrappedDefinition<THandler, HookOptions>;

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

// Schema for hook options (concurrency only)
const HookOptionsSchema = z.object({
	concurrency: z.enum(["serial", "parallel", "readonly"]).optional(),
});

// Schema for a hook that can be either a function or a wrapped definition
const hookSchema = z
	.union([
		zFunction(),
		z.object({
			handler: zFunction(),
			options: HookOptionsSchema,
		}),
	])
	.optional();

// Schema for action options (concurrency + timeout)
const ActionOptionsSchema = z.object({
	timeout: z.number().positive().optional(),
	concurrency: z.enum(["serial", "parallel", "readonly"]).optional(),
});

// Schema for an action that can be either a function or a wrapped definition
const ActionSchema = z.union([
	zFunction(),
	z.object({
		handler: zFunction(),
		options: ActionOptionsSchema,
	}),
]);

// This schema is used to validate the input at runtime. The generic types are defined below in `ActorConfig`.
//
// We don't use Zod generics with `z.custom` because:
// (a) there seems to be a weird bug in either Zod, tsup, or TSC that causese external packages to have different types from `z.infer` than from within the same package and
// (b) it makes the type definitions incredibly difficult to read as opposed to vanilla TypeScript.
export const ActorConfigSchema = z
	.object({
		onCreate: hookSchema,
		onDestroy: hookSchema,
		onWake: hookSchema,
		onSleep: hookSchema,
		onStateChange: hookSchema,
		onBeforeConnect: hookSchema,
		onConnect: hookSchema,
		onDisconnect: hookSchema,
		onBeforeActionResponse: hookSchema,
		onRequest: hookSchema,
		onWebSocket: hookSchema,
		actions: z.record(z.string(), ActionSchema).default(() => ({})),
		state: z.any().optional(),
		createState: zFunction().optional(),
		connState: z.any().optional(),
		createConnState: zFunction().optional(),
		vars: z.any().optional(),
		db: z.any().optional(),
		createVars: zFunction().optional(),
		options: z
			.object({
				createVarsTimeout: z.number().positive().default(5000),
				createConnStateTimeout: z.number().positive().default(5000),
				onConnectTimeout: z.number().positive().default(5000),
				// This must be less than ACTOR_STOP_THRESHOLD_MS
				onSleepTimeout: z.number().positive().default(5000),
				onDestroyTimeout: z.number().positive().default(5000),
				stateSaveInterval: z.number().positive().default(10_000),
				actionTimeout: z.number().positive().default(60_000),
				// Max time to wait for waitUntil background promises during shutdown
				waitUntilTimeout: z.number().positive().default(15_000),
				connectionLivenessTimeout: z.number().positive().default(2500),
				connectionLivenessInterval: z.number().positive().default(5000),
				noSleep: z.boolean().default(false),
				sleepTimeout: z.number().positive().default(30_000),
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
type CreateState<TState, TConnParams, TConnState, TVars, TInput, TDatabase> =
	| { state: TState }
	| {
			createState: (
				c: InitContext,
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
> =
	| { connState: TConnState }
	| {
			createConnState: (
				c: CreateConnStateContext<TState, TVars, TInput, TDatabase>,
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
type CreateVars<TState, TConnParams, TConnState, TVars, TInput, TDatabase> =
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
				c: InitContext,
				driverCtx: any,
			) => TVars | Promise<TVars>;
	  }
	| Record<never, never>;

/**
 * Type for an action handler function.
 */
export type ActionHandler<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> = (
	c: ActionContext<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
	...args: any[]
) => any;

/**
 * Type for an action entry - either a raw handler function or a wrapped ActionDefinition.
 */
export type ActionEntry<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> =
	| ActionHandler<TState, TConnParams, TConnState, TVars, TInput, TDatabase>
	| ActionDefinition<
			ActionHandler<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>
	  >;

export interface Actions<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> {
	[Action: string]: ActionEntry<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>;
}

/**
 * Type for a hook entry - either a raw handler function or a wrapped HookDefinition.
 */
export type HookEntry<THandler extends (...args: any[]) => any> =
	| THandler
	| HookDefinition<THandler>;

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
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>,
> {
	/**
	 * Called when the actor is first initialized.
	 *
	 * Use this hook to initialize your actor's state.
	 * This is called before any other lifecycle hooks.
	 */
	onCreate?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			input: TInput,
		) => void | Promise<void>
	>;

	/**
	 * Called when the actor is destroyed.
	 */
	onDestroy?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => void | Promise<void>
	>;

	/**
	 * Called when the actor is started and ready to receive connections and action.
	 *
	 * Use this hook to initialize resources needed for the actor's operation
	 * (timers, external connections, etc.)
	 *
	 * @returns Void or a Promise that resolves when startup is complete
	 */
	onWake?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => void | Promise<void>
	>;

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
	onSleep?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => void | Promise<void>
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
	onStateChange?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			newState: TState,
		) => void
	>;

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
	onBeforeConnect?: HookEntry<
		(
			c: OnBeforeConnectContext<TState, TVars, TInput, TDatabase>,
			params: TConnParams,
		) => void | Promise<void>
	>;

	/**
	 * Called when a client successfully connects to the actor.
	 *
	 * Use this hook to perform actions when a connection is established,
	 * such as sending initial data or updating the actor's state.
	 *
	 * @param conn The connection object
	 * @returns Void or a Promise that resolves when connection handling is complete
	 */
	onConnect?: HookEntry<
		(
			c: OnConnectContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			conn: Conn<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => void | Promise<void>
	>;

	/**
	 * Called when a client disconnects from the actor.
	 *
	 * Use this hook to clean up resources associated with the connection
	 * or update the actor's state.
	 *
	 * @param conn The connection that is being closed
	 * @returns Void or a Promise that resolves when disconnect handling is complete
	 */
	onDisconnect?: HookEntry<
		(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			conn: Conn<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
		) => void | Promise<void>
	>;

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
	onBeforeActionResponse?: HookEntry<
		<Out>(
			c: ActorContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			name: string,
			args: unknown[],
			output: Out,
		) => Out | Promise<Out>
	>;

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
	onRequest?: HookEntry<
		(
			c: RequestContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			request: Request,
		) => Response | Promise<Response>
	>;

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
	onWebSocket?: HookEntry<
		(
			c: WebSocketContext<
				TState,
				TConnParams,
				TConnState,
				TVars,
				TInput,
				TDatabase
			>,
			websocket: UniversalWebSocket,
		) => void | Promise<void>
	>;

	actions: TActions;
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
> = Omit<
	z.infer<typeof ActorConfigSchema>,
	| "actions"
	| "onCreate"
	| "onDestroy"
	| "onWake"
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
		Actions<TState, TConnParams, TConnState, TVars, TInput, TDatabase>
	> &
	CreateState<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	CreateConnState<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	CreateVars<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	ActorDatabaseConfig<TDatabase>;

// See description on `ActorConfig`
export type ActorConfigInput<
	TState = undefined,
	TConnParams = undefined,
	TConnState = undefined,
	TVars = undefined,
	TInput = undefined,
	TDatabase extends AnyDatabaseProvider = undefined,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
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
	| "onCreate"
	| "onDestroy"
	| "onWake"
	| "onSleep"
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
		TActions
	> &
	CreateState<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	CreateConnState<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	CreateVars<TState, TConnParams, TConnState, TVars, TInput, TDatabase> &
	ActorDatabaseConfig<TDatabase>;

/**
 * Wraps an action handler with configuration options.
 *
 * @example
 * ```ts
 * actor({
 *   actions: {
 *     foo: action((c, arg1: number) => { ... }, { timeout: 30000 })
 *   }
 * })
 * ```
 *
 * @param handler The action handler function
 * @param options Configuration options for this action
 * @returns An ActionDefinition that can be used in the actions object
 */
export function action<THandler extends (...args: any[]) => any>(
	handler: THandler,
	options: ActionOptions = {},
): ActionDefinition<THandler> {
	return {
		[WRAPPED_DEFINITION_SYMBOL]: true,
		handler,
		options,
	};
}

/**
 * Wraps a hook handler with configuration options.
 *
 * @example
 * ```ts
 * actor({
 *   onCreate: handler((c, input) => { ... }, { concurrency: "serial" }),
 *   onConnect: handler((c, conn) => { ... }, { concurrency: "parallel" }),
 * })
 * ```
 *
 * @param fn The hook handler function
 * @param options Configuration options for this hook
 * @returns A HookDefinition that can be used for lifecycle/connection hooks
 */
export function handler<THandler extends (...args: any[]) => any>(
	fn: THandler,
	options: HookOptions = {},
): HookDefinition<THandler> {
	return {
		[WRAPPED_DEFINITION_SYMBOL]: true,
		handler: fn,
		options,
	};
}

// For testing type definitions:
export function test<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TActions
	>,
): ActorConfig<TState, TConnParams, TConnState, TVars, TInput, TDatabase> {
	const config = ActorConfigSchema.parse(input) as ActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>;
	return config;
}
