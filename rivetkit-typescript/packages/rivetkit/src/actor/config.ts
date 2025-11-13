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

// This schema is used to validate the input at runtime. The generic types are defined below in `ActorConfig`.
//
// We don't use Zod generics with `z.custom` because:
// (a) there seems to be a weird bug in either Zod, tsup, or TSC that causese external packages to have different types from `z.infer` than from within the same package and
// (b) it makes the type definitions incredibly difficult to read as opposed to vanilla TypeScript.
export const ActorConfigSchema = z
	.object({
		onCreate: z.function().optional(),
		onDestroy: z.function().optional(),
		onWake: z.function().optional(),
		onSleep: z.function().optional(),
		onStateChange: z.function().optional(),
		onBeforeConnect: z.function().optional(),
		onConnect: z.function().optional(),
		onDisconnect: z.function().optional(),
		onBeforeActionResponse: z.function().optional(),
		onRequest: z.function().optional(),
		onWebSocket: z.function().optional(),
		actions: z.record(z.function()).default({}),
		state: z.any().optional(),
		createState: z.function().optional(),
		connState: z.any().optional(),
		createConnState: z.function().optional(),
		vars: z.any().optional(),
		db: z.any().optional(),
		createVars: z.function().optional(),
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
				/** @experimental */
				canHibernateWebSocket: z
					.union([
						z.boolean(),
						z
							.function()
							.args(z.custom<Request>())
							.returns(z.boolean()),
					])
					.default(false),
			})
			.strict()
			.default({}),
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

export interface Actions<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
> {
	[Action: string]: (
		c: ActionContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
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
	onCreate?: (
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		input: TInput,
	) => void | Promise<void>;

	/**
	 * Called when the actor is destroyed.
	 */
	onDestroy?: (
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
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
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
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
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
	) => void | Promise<void>;

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
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
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
		c: OnBeforeConnectContext<TState, TVars, TInput, TDatabase>,
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
		c: OnConnectContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
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
		c: ActorContext<
			TState,
			TConnParams,
			TConnState,
			TVars,
			TInput,
			TDatabase
		>,
		conn: Conn<TState, TConnParams, TConnState, TVars, TInput, TDatabase>,
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
			TDatabase
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
			TDatabase
		>,
		websocket: UniversalWebSocket,
	) => void | Promise<void>;

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
