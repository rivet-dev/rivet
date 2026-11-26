import {
	Context,
	type DateTime,
	type Duration,
	Effect,
	identity,
	Layer,
	type Option,
	Predicate,
	Schema,
	type Scope,
	type Stream,
	type Types,
} from "effect";
import type { HttpServerRequest } from "effect/http";
import type { Socket } from "effect/socket";
import type { SqlClient } from "effect/sql";
import type * as Rivetkit from "rivetkit";
import type * as RivetkitClient from "rivetkit/client";
import type * as RivetkitDb from "rivetkit/db";
import type * as Action from "./Action.ts";
import * as Client from "./Client.ts";
import type * as Event from "./Event.ts";
import type * as ActionDispatcher from "./internal/ActionDispatcher.ts";
import type * as ActorCronInternal from "./internal/ActorCron.ts";
import type * as ConnectionHooks from "./internal/ConnectionHooks.ts";
import type * as RawHttpHandler from "./internal/RawHttpHandler.ts";
import type * as RawWebSocketHandler from "./internal/RawWebSocketHandler.ts";
import { RegistryService } from "./internal/RegistryService.ts";
import type {
	Options,
	RivetkitActorDurationOptionKey,
} from "./internal/RivetkitActorDefinition.ts";
import type * as StateOptions from "./internal/StateOptions.ts";
import type * as Registry from "./Registry.ts";
import type * as RivetError from "./RivetError.ts";
import type * as State from "./State.ts";
import type * as Workflow from "./Workflow.ts";

const TypeId = "~@rivetkit/effect/Actor";

export const isActor = (u: unknown): u is Actor<any, any, any, any> =>
	Predicate.hasProperty(u, TypeId);

/**
 * `"workflow"` for a contract of `Workflow.make`, whose `toLayer` takes the
 * workflow body. `"actor"` for a contract of `Actor.make`.
 */
export type ContractKind = "actor" | "workflow";

type RivetkitActorOptionsInput = NonNullable<Rivetkit.ActorOptionsInput>;

type RivetkitActorOptionKey<Key extends keyof RivetkitActorOptionsInput> = Key;

type DeprecatedRivetkitActorOptionKey = RivetkitActorOptionKey<
	| "noSleep"
	| "stateSaveInterval"
	| "createConnStateTimeout"
	| "maxQueueSize"
	| "maxQueueMessageSize"
	| "onDestroyTimeout"
	| "waitUntilTimeout"
	| "preloadMaxWorkflowBytes"
	| "preloadMaxConnectionsBytes"
>;

/**
 * This bounds `createVars`. The SDK owns the RivetKit actor definition and
 * does not install it.
 */
type UnusedRivetkitActorOptionKey = RivetkitActorOptionKey<"createVarsTimeout">;

/**
 * RivetKit actor options forwarded to `Rivetkit.actor`. Timeouts and
 * intervals take a finite `Duration.Input` (for example `"5 minutes"`; a bare
 * number is milliseconds). Deprecated RivetKit options are left out.
 */
export type RivetkitActorOptions = Omit<
	RivetkitActorOptionsInput,
	| DeprecatedRivetkitActorOptionKey
	| UnusedRivetkitActorOptionKey
	| RivetkitActorDurationOptionKey
> & {
	readonly [Key in RivetkitActorDurationOptionKey]?: Duration.Input;
};

type StatelessOptions<
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
> = Readonly<RivetkitActorOptions> & {
	readonly state?: never;
	readonly db?: Database;
};

type StatefulOptions<
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
> = Readonly<RivetkitActorOptions> & {
	/** @deprecated Actor state was deprecated in 3.0 and will be removed in a future version. Store durable data in SQLite (`c.db`) and in-memory data in `c.vars`. See https://rivet.dev/actors/docs/deprecated/state */
	readonly state: State;
	readonly db?: Database;
};

/**
 * Per-instance identity carried inside the wake scope. An actor
 * instance is addressable in two ways:
 *
 * - `(name, key)` — stable user-facing pair (e.g. "Counter", ["counter-123"])
 * - `actorId` — opaque engine-assigned unique identifier
 *
 * `region` is the region the instance runs in. `namespace` is the namespace
 * it runs in: after a namespace fork it differs from the source actor's
 * namespace while the `actorId` stays the same. `forkChain` lists the actor
 * forks behind this actor, oldest first, and is empty for an actor that was
 * never forked.
 *
 * Available inside `Actor.toLayer`'s wake effect via
 * `yield* Actor.CurrentAddress`.
 */
export type ActorAddress = Pick<
	Rivetkit.ActorContext<any, any, any, any, any, any, any, any>,
	"actorId" | "name" | "key" | "region" | "namespace" | "forkChain"
>;

/**
 * Context tag for the current actor instance's address. Provided
 * once per wake when the wake effect runs; capture it into a
 * closure if action handlers need it.
 */
export class CurrentAddress extends Context.Service<
	CurrentAddress,
	ActorAddress
>()("@rivetkit/effect/Actor/CurrentAddress") {}

export class Sleep extends Context.Service<Sleep, Effect.Effect<void>>()(
	"@rivetkit/effect/Actor/Sleep",
) {}

/**
 * Puts the current actor instance to sleep. Teardown is asynchronous. The next
 * call to the actor wakes it again.
 */
export const sleep: Effect.Effect<void, never, Sleep> = Sleep.use(
	(sleep) => sleep,
);

/**
 * Context tag for the current actor instance's keep-awake function. Provided
 * once per wake and in every action handler. Use `Actor.keepAwake` instead of
 * reading it directly.
 */
export class KeepAwake extends Context.Service<
	KeepAwake,
	<A, E, R>(effect: Effect.Effect<A, E, R>) => Effect.Effect<A, E, R>
>()("@rivetkit/effect/Actor/KeepAwake") {}

/**
 * Keeps the actor from idle sleep until `effect` succeeds, fails or is
 * interrupted. Use it in the wake effect or in an action handler, for example
 * on work forked into the wake scope that must finish before the actor sleeps.
 */
export const keepAwake = <A, E, R>(
	effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, E, R | KeepAwake> =>
	KeepAwake.use((keepAwake) => keepAwake(effect));

/**
 * Context tag for the effect that destroys the current actor instance.
 * Provided once per wake and in every action handler.
 */
export class Destroy extends Context.Service<Destroy, Effect.Effect<void>>()(
	"@rivetkit/effect/Actor/Destroy",
) {}

/**
 * Destroys the current actor instance and deletes its state. Teardown is
 * asynchronous. Once it finishes, a `getOrCreate` with the same key creates a
 * new actor.
 */
export const destroy: Effect.Effect<void, never, Destroy> = Destroy.use(
	(destroy) => destroy,
);

const EventsTypeId = "~@rivetkit/effect/Actor/Events";

/**
 * The requirement of `MyActor.events`. Each actor has its own, so only that
 * actor's wake effect and action handlers provide it.
 */
export interface Events<Name extends string> {
	readonly [EventsTypeId]: Name;
}

/**
 * The connection that made an action call. The server sets `id` and `scope`,
 * so a client cannot forge them.
 *
 * The id is an address, not an identity. Clients can read their own id, so do
 * not use it for authorization.
 *
 * `params` exists only when the actor declares `connection`. Clients supply
 * them, the actor decodes them once per connection and `authorize` checks them.
 */
export type ActionConnection<Params = never> = Readonly<
	Pick<Rivetkit.AnyConn, "id">
> &
	([Params] extends [never] ? unknown : { readonly params: Params }) & {
		/**
		 * Closes when the connection disconnects or the actor sleeps or is
		 * destroyed. Bind per-connection resources to it. If the connection has
		 * already closed, finalizers added to this scope run immediately.
		 *
		 * Stateless calls, including calls from `Actor.client`, use a connection
		 * that closes as soon as the action returns.
		 */
		readonly scope: Scope.Scope;
	};

export type ActionRequest<
	A extends Action.Any,
	Params = never,
> = A extends Action.Action<
	infer Tag,
	infer Payload,
	infer _Success,
	infer _Error
>
	? {
			readonly _tag: Tag;
			readonly action: A;
			readonly payload: Payload["Type"];
			/** `None` when the action was started by the actor's schedule. */
			readonly connection: Option.Option<ActionConnection<Params>>;
		}
	: never;

/**
 * What an `onBeforeActionResponse` hook gets: the action request and the
 * success its handler returned.
 */
export type ActionResponse<
	A extends Action.Any,
	Params = never,
> = A extends Action.Any
	? ActionRequest<A, Params> & { readonly success: Action.Success<A> }
	: never;

type SchedulePayloadArgs<A extends Action.Any> = [
	Action.PayloadSchema<A>,
] extends [Schema.Void]
	? []
	: [payload: Action.PayloadConstructor<A>];

type ScheduleServices<A extends Action.Any> =
	Action.PayloadSchema<A>["EncodingServices"];

/** A pending call from `ActorSchedule`, with its payload decoded. */
export type ScheduledCall<A extends Action.Any> =
	A extends Action.Action<
		infer Tag,
		infer Payload,
		infer _Success,
		infer _Error
	>
		? {
				readonly id: string;
				readonly _tag: Tag;
				readonly payload: Payload["Type"];
				readonly runAt: DateTime.Utc;
			}
		: never;

type ScheduledCallServices<Actions extends Action.Any> =
	Action.PayloadSchema<Actions>["DecodingServices"];

/**
 * A stored scheduled call that this actor cannot decode: its action is not one
 * of the actor's actions, or its payload fails the action's payload schema.
 */
export class ScheduledCallDecodeError extends Schema.TaggedError<ScheduledCallDecodeError>()(
	"ScheduledCallDecodeError",
	{
		id: Schema.String,
		action: Schema.String,
		cause: Schema.optional(Schema.instanceOf(Schema.SchemaError)),
	},
) {
	override get message() {
		return `Scheduled call ${this.id} to ${this.action} does not decode`;
	}
}

/**
 * Schedules calls to this actor's own actions. RivetKit stores the call, so it
 * survives sleep, and runs it like a client call with `connection: None`.
 * Payloads are encoded with the action's JSON codec, the same as
 * `Actor.client`. An encoding failure is a defect. RivetKit rejections, for
 * example a full `maxSchedules`, fail with a `RivetError`.
 *
 * `get` and `list` decode payloads with the same codec. A stored call can stop
 * decoding, for example after a payload schema change or when raw RivetKit
 * code scheduled it. `get` fails with a `ScheduledCallDecodeError` for it, and
 * `list` skips it and logs a warning.
 */
export interface ActorSchedule<Actions extends Action.Any> {
	/** Calls `action` after `delay`. Succeeds with the schedule id. */
	readonly after: <A extends Actions>(
		delay: Duration.Input,
		action: A,
		...payload: SchedulePayloadArgs<A>
	) => Effect.Effect<string, RivetError.RivetError, ScheduleServices<A>>;
	/** Calls `action` at `at`. Succeeds with the schedule id. */
	readonly at: <A extends Actions>(
		at: DateTime.DateTime | Date,
		action: A,
		...payload: SchedulePayloadArgs<A>
	) => Effect.Effect<string, RivetError.RivetError, ScheduleServices<A>>;
	/** Cancels a scheduled call. Succeeds with `false` if no pending call has this id. */
	readonly cancel: (
		id: string,
	) => Effect.Effect<boolean, RivetError.RivetError>;
	/** Succeeds with the pending call with this id, or `None`. */
	readonly get: (
		id: string,
	) => Effect.Effect<
		Option.Option<ScheduledCall<Actions>>,
		RivetError.RivetError | ScheduledCallDecodeError,
		ScheduledCallServices<Actions>
	>;
	/** Succeeds with all pending calls that decode. */
	readonly list: Effect.Effect<
		ReadonlyArray<ScheduledCall<Actions>>,
		RivetError.RivetError,
		ScheduledCallServices<Actions>
	>;
}

const ScheduleTypeId = "~@rivetkit/effect/Actor/Schedule";

/**
 * The requirement of `MyActor.schedule`. Each actor has its own, so only that
 * actor's wake effect and action handlers provide it.
 */
export interface Schedule<Name extends string> {
	readonly [ScheduleTypeId]: Name;
}

export {
	type ActorCron,
	type CronEveryOptions,
	type CronFire,
	type CronJob,
	CronJobDecodeError,
	type CronSetOptions,
} from "./internal/ActorCron.ts";

const CronTypeId = "~@rivetkit/effect/Actor/Cron";

/**
 * The requirement of `MyActor.cron`. Each actor has its own, so only that
 * actor's wake effect and action handlers provide it.
 */
export interface Cron<Name extends string> {
	readonly [CronTypeId]: Name;
}

/**
 * Declares the params a client connects with and the error `authorize` can
 * refuse a connection with.
 */
export interface ConnectionContract<
	Params extends Schema.Top = Schema.Top,
	Error extends Schema.Top = Schema.Top,
> {
	readonly params: Params;
	readonly error: Error;
}

export type AnyConnectionContract = ConnectionContract<Schema.Top, Schema.Top>;

/** `never` when the actor declares no `connection`. */
export type ConnectionParams<Connection extends AnyConnectionContract> = [
	Connection,
] extends [never]
	? never
	: Connection["params"]["Type"];

/** `never` when the actor declares no `connection`. */
export type ConnectionError<Connection extends AnyConnectionContract> = [
	Connection,
] extends [never]
	? never
	: Connection["error"]["Type"];

type ConnectionServicesServer<Connection extends AnyConnectionContract> = [
	Connection,
] extends [never]
	? never
	:
			| Connection["params"]["DecodingServices"]
			| Connection["error"]["EncodingServices"];

type ConnectionServicesClient<Connection extends AnyConnectionContract> = [
	Connection,
] extends [never]
	? never
	:
			| ConnectionParamsEncodingServices<Connection>
			| Connection["error"]["DecodingServices"];

type ConnectionParamsEncodingServices<
	Connection extends AnyConnectionContract,
> = [Connection] extends [never]
	? never
	: Connection["params"]["EncodingServices"];

/** `undefined` when the actor declares no `input`. */
export type CreationInput<Input extends Schema.Top> = [Input] extends [never]
	? undefined
	: Input["Type"];

type CreationInputEncodingServices<Input extends Schema.Top> = [Input] extends [
	never,
]
	? never
	: Input["EncodingServices"];

type CreationInputDecodingServices<Input extends Schema.Top> = [Input] extends [
	never,
]
	? never
	: Input["DecodingServices"];

/**
 * Reads and closes the open connections of the current actor instance. Read
 * it with `yield* MyActor.connections`.
 */
export type ActorConnections<Connection extends AnyConnectionContract> = {
	/**
	 * Succeeds with the open connections. A stateless call counts while it
	 * runs. A connection whose params no longer decode is skipped with a
	 * logged warning.
	 */
	readonly list: Effect.Effect<
		ReadonlyArray<ActionConnection<ConnectionParams<Connection>>>
	>;
	/** Closes a connection. Does nothing if it has already closed. */
	readonly disconnect: (
		connection: ActionConnection,
		reason?: string,
	) => Effect.Effect<void, RivetError.RivetError>;
};

type ConnectionHook<Connection extends AnyConnectionContract, R> = (
	connection: ActionConnection<ConnectionParams<Connection>>,
) => Effect.Effect<void, never, R>;

/**
 * The hooks that run while the actor is awake, passed to `MyActor.of` with
 * the action handlers. They run as fibers in the actor instance, only after
 * the wake effect completes, with the services of the wake effect.
 */
export type Hooks<
	Actions extends Action.Any,
	Connection extends AnyConnectionContract,
	RAuthorize = never,
	RConnect = never,
	RDisconnect = never,
	RHttp = never,
	RWebSocket = never,
	RActionResponse = never,
	EventDefs extends Event.Any = never,
	RCanSubscribe = never,
	RDestroy = never,
> = {
	/**
	 * Runs when the actor is destroyed while it is awake, for example with
	 * `Actor.destroy` or a destroy from a client. It runs after `onDisconnect`
	 * for each closing connection and before the wake scope closes, so the
	 * values of the wake effect and the database still work. RivetKit stops
	 * waiting for it after `sleepGracePeriod`. A failure or defect is logged
	 * and the actor is destroyed anyway.
	 *
	 * It never runs on sleep. It is also best effort: RivetKit destroys a
	 * sleeping actor without waking it, and a crash or a destroy during the
	 * sleep grace period runs no `onDestroy`. So do not rely on it for a
	 * delete that must happen. To release what one wake opened, on sleep and
	 * on destroy, add a finalizer to the wake scope instead.
	 *
	 * @example
	 * ```ts
	 * onDestroy: Effect.gen(function* () {
	 *   yield* upstream.deleteRoom;
	 * })
	 * ```
	 */
	readonly onDestroy?: Effect.Effect<void, never, RDestroy>;
	/**
	 * Decides whether a connection can subscribe to an event. Each key is an
	 * event tag. The hook runs before each subscription, with the connection
	 * that subscribes. When it succeeds with `false`, RivetKit refuses the
	 * subscription: it closes that connection with `auth.forbidden`, and each
	 * event stream of that connection fails with that `RivetError`. A defect
	 * also refuses the subscription. An event without a hook allows every
	 * connection.
	 *
	 * @example
	 * ```ts
	 * canSubscribe: {
	 *   ScoreChanged: Effect.fn("Game.canSubscribe.ScoreChanged")(function* (
	 *     connection,
	 *   ) {
	 *     return connection.params.role === "player";
	 *   }),
	 * }
	 * ```
	 */
	readonly canSubscribe?: {
		readonly [E in EventDefs as E["_tag"]]?: (
			connection: ActionConnection<ConnectionParams<Connection>>,
		) => Effect.Effect<boolean, never, RCanSubscribe>;
	};
	/**
	 * Replaces the success of an action before the caller gets it. Each key is
	 * an action tag. The hook runs after that action's handler succeeds, with
	 * the handler's services, and the value it succeeds with is encoded with
	 * the action's success schema. It does not run when the handler fails. A
	 * defect fails the call.
	 *
	 * @example
	 * ```ts
	 * onBeforeActionResponse: {
	 *   GetProfile: Effect.fn("User.GetProfile.response")(function* ({
	 *     success,
	 *   }) {
	 *     return { ...success, email: undefined };
	 *   }),
	 * }
	 * ```
	 */
	readonly onBeforeActionResponse?: {
		readonly [A in Actions as A["_tag"]]?: (
			response: ActionResponse<A, ConnectionParams<Connection>>,
		) => Effect.Effect<Action.Success<A>, never, RActionResponse>;
	};
	/** Runs once a connection opens, after `authorize` accepted it. */
	readonly onConnect?: ConnectionHook<Connection, RConnect>;
	/** Runs once a connection closes while the actor is awake. */
	readonly onDisconnect?: ConnectionHook<Connection, RDisconnect>;
	/**
	 * Handles the raw HTTP requests sent to the actor, for example with
	 * `handle.fetch`. Runs once per request, with the request as
	 * `HttpServerRequest`. A failure becomes a `500` response.
	 *
	 * @example
	 * ```ts
	 * http: Effect.gen(function* () {
	 *   const request = yield* HttpServerRequest.HttpServerRequest;
	 *   const sql = yield* Notes.sql;
	 *   const rows = yield* sql`SELECT text FROM notes`;
	 *   return yield* HttpServerResponse.json({ url: request.url, rows });
	 * })
	 * ```
	 */
	readonly http?: RawHttpHandler.HttpHandler<RHttp>;
	/**
	 * Handles each raw WebSocket connection to the actor, for example from
	 * `handle.webSocket`. Runs with the connection as `Socket` and the upgrade
	 * request as `HttpServerRequest`.
	 *
	 * The connection closes when the handler exits: normally on success, with
	 * code `1011` on a failure. When the client disconnects or the actor
	 * sleeps, the handler is interrupted and its scope closes.
	 *
	 * @example
	 * ```ts
	 * webSocket: Effect.gen(function* () {
	 *   const socket = yield* Socket.Socket;
	 *   const { write } = yield* socket.writer;
	 *   const pull = yield* Socket.readerString(socket);
	 *   yield* pull.pipe(
	 *     Effect.flatMap(Effect.forEach(write)),
	 *     Effect.forever,
	 *   );
	 * })
	 * ```
	 */
	readonly webSocket?: RawWebSocketHandler.WebSocketHandler<RWebSocket>;
} & ([Connection] extends [never]
	? { readonly authorize?: never }
	: {
			/**
			 * Runs before a connection opens, with its decoded params. A failure
			 * refuses the connection, and the client gets the declared
			 * `connection.error`. Stateless calls open a connection too.
			 */
			readonly authorize: (
				params: ConnectionParams<Connection>,
				request: Option.Option<Request>,
			) => Effect.Effect<void, ConnectionError<Connection>, RAuthorize>;
		});

type AnyHooks = ConnectionHooks.UnboundConnectionHooks & {
	readonly onBeforeActionResponse?: ActionDispatcher.ActionResponseHooks;
	readonly http?: RawHttpHandler.HttpHandler<any>;
	readonly webSocket?: RawWebSocketHandler.WebSocketHandler<any>;
	readonly onDestroy?: Effect.Effect<void, never, any>;
};

const ImplementationTypeId = "~@rivetkit/effect/Actor/Implementation";

/**
 * Action handlers together with hooks, as `MyActor.of(handlers, hooks)`
 * returns them. Return it from the wake effect.
 */
export interface Implementation<ActionHandlers, HookServices = never> {
	readonly [ImplementationTypeId]: {
		readonly _HookServices: Types.Covariant<HookServices>;
	};
	readonly actionHandlers: ActionHandlers;
	readonly hooks: AnyHooks;
}

/** @internal */
export const isImplementation = (u: unknown): u is Implementation<any, any> =>
	Predicate.hasProperty(u, ImplementationTypeId);

/** An actor that declares `connection` needs `authorize`, so it needs hooks. */
type HandlersOrImplementation<
	ActionHandlers,
	HookServices,
	Connection extends AnyConnectionContract,
> =
	| Implementation<ActionHandlers, HookServices>
	| ([Connection] extends [never] ? ActionHandlers : never);

const ConnectionsTypeId = "~@rivetkit/effect/Actor/Connections";

/**
 * The requirement of `MyActor.connections`. Each actor has its own, so only
 * that actor's wake effect and action handlers provide it.
 */
export interface Connections<Name extends string> {
	readonly [ConnectionsTypeId]: Name;
}

const SqlTypeId = "~@rivetkit/effect/Actor/Sql";

/**
 * The requirement of `MyActor.sql`. Each actor has its own, so only that
 * actor's layer provides it, and only when the actor declares `db`.
 */
export interface Sql<Name extends string> {
	readonly [SqlTypeId]: Name;
}

type EventPayloadArgs<E extends Event.Any> = [Event.PayloadSchema<E>] extends [
	Schema.Void,
]
	? []
	: [payload: Event.PayloadConstructor<E>];

type EventServices<E extends Event.Any> =
	Event.PayloadSchema<E>["EncodingServices"];

/**
 * Pushes events to the actor's connections. Payloads are encoded with the
 * event's JSON codec. An encoding failure is a defect.
 */
export interface ActorEvents<EventDefs extends Event.Any> {
	/** Sends `event` to every connection. */
	readonly broadcast: <E extends EventDefs>(
		event: E,
		...payload: EventPayloadArgs<E>
	) => Effect.Effect<void, never, EventServices<E>>;
	/** Sends `event` to one connection. Does nothing if it has closed. */
	readonly send: <E extends EventDefs>(
		connection: ActionConnection,
		event: E,
		...payload: EventPayloadArgs<E>
	) => Effect.Effect<void, never, EventServices<E>>;
}

type ActionHandlerServices<ActionHandlers> = {
	readonly [Name in keyof ActionHandlers]: ActionHandlers[Name] extends (
		...args: ReadonlyArray<any>
	) => Effect.Effect<any, any, infer R>
		? R
		: never;
}[keyof ActionHandlers];

type RivetkitActorDefinitionFor<
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = Rivetkit.ActorDefinition<
	StateOptions.Encoded<State>,
	undefined,
	undefined,
	undefined,
	undefined,
	Database,
	Record<never, never>,
	Record<never, never>,
	any
>;

export type WakeOptions<
	ActorDefinition extends
		Rivetkit.AnyActorDefinition = Rivetkit.AnyActorDefinition,
> = {
	readonly rawRivetkitContext: Rivetkit.WakeContextOf<ActorDefinition>;
};

type RawWakeContextFor<
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = {
	[Key in keyof Rivetkit.WakeContextOf<
		RivetkitActorDefinitionFor<State, Database>
	>]: Key extends "state"
		? [State] extends [never]
			? never
			: StateOptions.Encoded<State>
		: Rivetkit.WakeContextOf<
				RivetkitActorDefinitionFor<State, Database>
			>[Key];
};

type WakeOptionsFor<
	StateDefinition extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = {
	readonly rawRivetkitContext: RawWakeContextFor<StateDefinition, Database>;
} & ([StateDefinition] extends [never]
	? unknown
	: {
			/** @deprecated Actor state was deprecated in 3.0 and will be removed in a future version. Store durable data in SQLite (`c.db`) and in-memory data in `c.vars`. See https://rivet.dev/actors/docs/deprecated/state */
			readonly state: State.State<
				StateOptions.Decoded<StateDefinition>,
				Schema.SchemaError,
				StateOptions.Services<StateDefinition>
			>;
		});

type WakeFunction<ActionHandlers, R, W extends WakeOptions> =
	| ((wakeOptions: W) => ActionHandlers)
	| ((wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>);

type Wake<ActionHandlers, R, RX, W extends WakeOptions> =
	| ActionHandlers
	| Effect.Effect<ActionHandlers, never, RX>
	| WakeFunction<ActionHandlers, R, W>
	| Effect.Effect<WakeFunction<ActionHandlers, R, W>, never, RX>;

export type AccessorKeyParam = string | Rivetkit.ActorKey;

/**
 * `MemberName` are the other members of the object. A member wins over an
 * action with the same tag, as in the RivetKit client.
 */
type ActionMethods<
	Actions extends Action.Any,
	Connection extends AnyConnectionContract,
	HandleServices = never,
	MemberName extends string = never,
> = {
	readonly [A in Actions as Exclude<Action.Tag<A>, MemberName>]: (
		payload: Action.PayloadConstructor<A>,
	) => Effect.Effect<
		Action.Success<A>,
		Action.Error<A> | ConnectionError<Connection> | RivetError.RivetError,
		| Action.ServicesClient<A>
		| ConnectionServicesClient<Connection>
		| HandleServices
	>;
};

/**
 * A typed handle for one actor instance. Each action becomes a
 * method that takes the action's payload-constructor input and
 * returns an Effect with the action's success / typed error
 * channels baked in.
 */
export type Handle<
	Actions extends Action.Any,
	EventDefs extends Event.Any = never,
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
> = ActionMethods<
	Actions,
	Connection,
	CreationInputEncodingServices<Input>,
	| "connect"
	| keyof RawHandleMethods<never>
	| keyof ResolveHandleMethods<never>
> & {
	/**
	 * Opens a connection that closes with the scope. Fails with the declared
	 * `connection.error` when `authorize` refuses it.
	 */
	readonly connect: Effect.Effect<
		ClientConnection<Actions, EventDefs, Connection>,
		ConnectionError<Connection> | RivetError.RivetError,
		| Scope.Scope
		| ConnectionServicesClient<Connection>
		| CreationInputEncodingServices<Input>
	>;
} & RawHandleMethods<Connection, CreationInputEncodingServices<Input>> &
	ResolveHandleMethods<Connection, CreationInputEncodingServices<Input>>;

/**
 * Members that resolve the handle to its actor. A handle from `getOrCreate`
 * creates the actor if it does not exist.
 */
type ResolveHandleMethods<
	Connection extends AnyConnectionContract,
	HandleServices = never,
> = {
	/** Succeeds with the actor ID. */
	readonly resolve: Effect.Effect<
		string,
		RivetError.RivetError,
		ConnectionParamsEncodingServices<Connection> | HandleServices
	>;
	/** Succeeds with the raw gateway URL that routes traffic to the actor. */
	readonly getGatewayUrl: Effect.Effect<
		string,
		RivetError.RivetError,
		ConnectionParamsEncodingServices<Connection> | HandleServices
	>;
	/**
	 * Issues a token scoped to this actor. Without `permissions`, it grants
	 * read access to the actor gateway only.
	 */
	readonly issueToken: (
		options?: RivetkitClient.ActorIssueTokenOptions,
	) => Effect.Effect<
		RivetkitClient.IssuedToken,
		RivetError.RivetError,
		ConnectionParamsEncodingServices<Connection> | HandleServices
	>;
};

type RivetkitFetchParameters = Parameters<
	RivetkitClient.ActorHandleRaw["fetch"]
>;

type RawHandleMethods<
	Connection extends AnyConnectionContract,
	HandleServices = never,
> = {
	/**
	 * Sends a raw HTTP request to the actor's `http` handler. `input` is a
	 * path, or a URL or `Request` whose path is used. Succeeds with the
	 * response for any status. Interruption aborts the request.
	 */
	readonly fetch: (
		input: RivetkitFetchParameters[0],
		init?: Omit<NonNullable<RivetkitFetchParameters[1]>, "signal">,
	) => Effect.Effect<
		Response,
		RivetError.RivetError,
		ConnectionParamsEncodingServices<Connection> | HandleServices
	>;
	/**
	 * A socket to the actor's `webSocket` handler. It connects when its
	 * reader is acquired and closes with that reader's scope. If the actor
	 * cannot be reached, the reader fails with a `SocketOpenError` whose
	 * cause is the `RivetError`.
	 */
	readonly webSocket: (
		path?: string,
		protocols?: string | Array<string>,
	) => Effect.Effect<
		Socket.Socket,
		never,
		ConnectionParamsEncodingServices<Connection> | HandleServices
	>;
};

/**
 * An open connection from `Handle.connect`. Action calls go over it, and
 * `events` has one stream of decoded payloads per declared event.
 */
export type ClientConnection<
	Actions extends Action.Any,
	EventDefs extends Event.Any = never,
	Connection extends AnyConnectionContract = never,
> = ActionMethods<Actions, Connection, never, "events"> & {
	/**
	 * A stream subscribes when it starts and ends when the connection closes.
	 * It fails if the connection drops and cannot reconnect, and with
	 * `auth.forbidden` when the actor's `canSubscribe` refuses a subscription
	 * of this connection.
	 */
	readonly events: {
		readonly [E in EventDefs as E["_tag"]]: Stream.Stream<
			Event.PayloadSchema<E>["Type"],
			RivetError.RivetError,
			Event.PayloadSchema<E>["DecodingServices"]
		>;
	};
};

type RivetkitGetOrCreateOptions = NonNullable<
	Parameters<RivetkitClient.ClientRaw["getOrCreate"]>[2]
>;

/**
 * `params` reach the actor as the connection params of each call. Only an
 * actor that declares `connection` takes them, encoded with its params schema.
 */
type ConnectionParamsOption<Connection extends AnyConnectionContract> = [
	Connection,
] extends [never]
	? { readonly params?: never }
	: { readonly params: ConnectionParams<Connection> };

/**
 * The creation input reaches the actor's `onCreate`. Only an actor that
 * declares `input` takes it, encoded with its input schema.
 */
type CreationInputOption<Input extends Schema.Top, Key extends string> = [
	Input,
] extends [never]
	? { readonly [K in Key]?: never }
	: { readonly [K in Key]: Input["Type"] };

/**
 * Options are required when the actor declares `connection`, or `input` for
 * the accessors that can create it.
 */
type AccessorOptionsArgs<
	Connection extends AnyConnectionContract,
	AccessorOptions,
	Input extends Schema.Top = never,
> = [Connection] extends [never]
	? [Input] extends [never]
		? [options?: AccessorOptions]
		: [options: AccessorOptions]
	: [options: AccessorOptions];

/**
 * Options for `Accessor.getOrCreate`. The actor gets `createWithInput` only
 * if this call creates it.
 */
export type GetOrCreateOptions<
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
> = Pick<RivetkitGetOrCreateOptions, "createInRegion" | "poolName"> &
	ConnectionParamsOption<Connection> &
	CreationInputOption<Input, "createWithInput">;

/** Options for `Accessor.get` and `Accessor.getForId`. */
export type GetOptions<Connection extends AnyConnectionContract = never> =
	ConnectionParamsOption<Connection>;

/** Options for `Accessor.create`. */
export type CreateOptions<
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
> = Pick<RivetkitClient.CreateOptions, "region" | "poolName"> &
	ConnectionParamsOption<Connection> &
	CreationInputOption<Input, "input">;

/**
 * Options for `Accessor.fork`. Omit `key` for a keyless actor. The new
 * actor's `onFork` gets `input`.
 */
export type ForkOptions<
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
> = {
	readonly key?: AccessorKeyParam;
} & ConnectionParamsOption<Connection> &
	CreationInputOption<Input, "input">;

/**
 * Yielded by `Actor.client`. Address an actor instance, then dispatch
 * typed action calls against the returned `Handle`.
 */
export type Accessor<
	Actions extends Action.Any,
	EventDefs extends Event.Any = never,
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
> = {
	/** Addresses the actor for `key`. The first call creates it if it does not exist. */
	readonly getOrCreate: (
		key: AccessorKeyParam,
		...options: AccessorOptionsArgs<
			Connection,
			GetOrCreateOptions<Connection, Input>,
			Input
		>
	) => Handle<Actions, EventDefs, Connection, Input>;
	/** Addresses an existing actor for `key`. Calls fail with `actor.not_found` if it does not exist. */
	readonly get: (
		key: AccessorKeyParam,
		...options: AccessorOptionsArgs<Connection, GetOptions<Connection>>
	) => Handle<Actions, EventDefs, Connection>;
	/** Addresses an existing actor by its actor ID. */
	readonly getForId: (
		actorId: string,
		...options: AccessorOptionsArgs<Connection, GetOptions<Connection>>
	) => Handle<Actions, EventDefs, Connection>;
	/** Creates a new actor for `key`. Fails if an actor for `key` already exists. */
	readonly create: (
		key: AccessorKeyParam,
		...options: AccessorOptionsArgs<
			Connection,
			CreateOptions<Connection, Input>,
			Input
		>
	) => Effect.Effect<
		Handle<Actions, EventDefs, Connection>,
		RivetError.RivetError,
		| ConnectionParamsEncodingServices<Connection>
		| CreationInputEncodingServices<Input>
	>;
	/**
	 * Forks the actor `actorId` into a new actor of this name. The new actor
	 * starts on a copy of the source's database as of its last committed
	 * write, runs `onFork` once and then runs independently of the source.
	 * A fork is not idempotent: a `key` that is already held fails with
	 * `actor.duplicate_key`.
	 */
	readonly fork: (
		actorId: string,
		...options: AccessorOptionsArgs<
			Connection,
			ForkOptions<Connection, Input>,
			Input
		>
	) => Effect.Effect<
		Handle<Actions, EventDefs, Connection>,
		RivetError.RivetError,
		| ConnectionParamsEncodingServices<Connection>
		| CreationInputEncodingServices<Input>
	>;
};

type UnknownToNever<T> = unknown extends T ? never : T;

/**
 * Only an actor that declares `db` gets `MyActor.sql` and a `SqlClient`.
 * Without one, they stay requirements of the layer.
 */
type BuiltInDatabaseServices<
	Name extends string,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = [Database] extends [undefined] ? never : SqlClient.SqlClient | Sql<Name>;

type BuiltInActionServices<
	Name extends string,
	Database extends RivetkitDb.AnyDatabaseProvider,
> =
	| CurrentAddress
	| Sleep
	| KeepAwake
	| Destroy
	| Schedule<Name>
	| Cron<Name>
	| Events<Name>
	| Connections<Name>
	| BuiltInDatabaseServices<Name, Database>;

/**
 * `onCreate` and `onMigrate` run before the wake effect, so they get only the
 * services that exist without an actor instance.
 */
type ExcludeBuiltInPreWakeServices<
	T,
	Name extends string,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = UnknownToNever<
	Exclude<T, CurrentAddress | BuiltInDatabaseServices<Name, Database>>
>;

type CreateHookOption<Input extends Schema.Top, RCreate> = {
	/**
	 * Runs once, when the actor is created, with the decoded creation input.
	 * It runs before the wake effect, with the layer's services,
	 * `Actor.CurrentAddress` and, for an actor with `db`, `MyActor.sql`. The
	 * wake scope and the per-wake services (`schedule`, `cron`, `events`,
	 * `connections`, `Actor.sleep`, `Actor.keepAwake`, `Actor.destroy`) do
	 * not exist yet. The input is not kept, so store what later wakes need.
	 * RivetKit stops waiting after 5 seconds.
	 *
	 * A defect, or input that does not decode with the actor's `input`
	 * schema, stops the actor before it wakes. RivetKit does not pass the
	 * cause to the caller, whose calls fail with
	 * `guard.actor_stopped_while_waiting`.
	 */
	readonly onCreate?: (
		input: CreationInput<Input>,
	) => Effect.Effect<void, never, RCreate>;
};

type MigrateHookOption<RMigrate> = {
	/**
	 * Runs on every actor start, after the `db` provider's own migrations and
	 * before `onCreate` and the wake effect. `isNew` is `true` on the start
	 * that creates the actor. Use it for repeatable schema migrations. It gets
	 * the same services as `onCreate`. A defect fails the start. RivetKit
	 * stops waiting after `onMigrateTimeout`.
	 */
	readonly onMigrate?: (options: {
		readonly isNew: boolean;
	}) => Effect.Effect<void, never, RMigrate>;
};

/**
 * The fork an `onFork` hook runs for: the newest entry of
 * `CurrentAddress.forkChain` and the fork input.
 */
export type Fork<Input extends Schema.Top = never> =
	ActorAddress["forkChain"][number] & {
		readonly input: CreationInput<Input>;
	};

type ForkHookOption<Input extends Schema.Top, RFork> = {
	/**
	 * Runs once, at the first start of a fork, in place of `onCreate`, with
	 * the source actor and the fork input decoded with the actor's `input`
	 * schema. It runs after `onMigrate` and before the wake effect, with the
	 * same services as `onCreate`. The database is a copy of the source's.
	 * A defect, or input that does not decode, fails the start of the fork.
	 * A namespace fork does not run it.
	 */
	readonly onFork?: (fork: Fork<Input>) => Effect.Effect<void, never, RFork>;
};

/**
 * Only a workflow contract takes `onError`. It runs in the actor instance,
 * so it gets the services of the wake effect.
 */
type WorkflowErrorHookOption<Kind extends ContractKind, ROnError> = [
	Kind,
] extends ["workflow"]
	? {
			/**
			 * Runs when a step attempt fails, when a rollback fails and when the
			 * workflow fails outside a step, with the event of
			 * `@rivet-dev/workflows`. It is not part of the workflow history,
			 * so it is not replayed. Use it for logs, metrics, error reports or
			 * data that the workflow does not read. A defect is logged and does
			 * not change the workflow.
			 */
			readonly onError?: (
				event: Workflow.ErrorEvent,
			) => Effect.Effect<void, never, ROnError>;
		}
	: { readonly onError?: never };

/**
 * The first argument of `toLayer`. For a workflow contract, it is an effect
 * that succeeds with the workflow body. See `Workflow.make`.
 */
type ToLayerWake<Kind extends ContractKind, ActorWake, RBody, RBodyBuilder> = [
	Kind,
] extends ["workflow"]
	? Effect.Effect<Effect.Effect<void, never, RBody>, never, RBodyBuilder>
	: ActorWake;

type WorkflowActionsOption<ActorWake> = {
	/**
	 * The action handlers and hooks of the workflow: `MyWorkflow.of(handlers)`,
	 * `MyWorkflow.of(handlers, hooks)`, or an effect that succeeds with one.
	 * It takes what the first argument of an actor's `toLayer` takes. An
	 * effect runs on each wake in the wake scope, after the effect that
	 * builds the workflow body.
	 */
	readonly actions: ActorWake;
};

/**
 * A workflow contract needs `actions` when it declares actions, or
 * `connection`, which needs `authorize`. An actor contract does not take it.
 */
type ActionsOption<
	Kind extends ContractKind,
	Actions extends Action.Any,
	Connection extends AnyConnectionContract,
	ActorWake,
> = [Kind] extends ["workflow"]
	? [Actions | Connection] extends [never]
		? Partial<WorkflowActionsOption<ActorWake>>
		: WorkflowActionsOption<ActorWake>
	: { readonly actions?: never };

/** `toLayer` needs options only for a workflow contract that needs `actions`. */
type RequiredOptionsArgs<
	Kind extends ContractKind,
	Actions extends Action.Any,
	Connection extends AnyConnectionContract,
	ActorWake,
> = [Kind] extends ["workflow"]
	? [Actions | Connection] extends [never]
		? []
		: [options: WorkflowActionsOption<ActorWake>]
	: [];

type ExcludeBuiltInWakeServices<
	T,
	Name extends string,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = UnknownToNever<
	Exclude<T, Scope.Scope | BuiltInActionServices<Name, Database>>
>;

/**
 * Handlers also run with the wake scope, but a handler that needs a scope
 * should own one with `Effect.scoped`, so `Scope` stays a requirement.
 */
type ExcludeBuiltInActionServices<
	T,
	Name extends string,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = UnknownToNever<Exclude<T, BuiltInActionServices<Name, Database>>>;

type ToLayerRequirements<
	Name extends string,
	Actions extends Action.Any,
	ActionHandlers,
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
	R,
	RX,
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
	RCreate = never,
	RMigrate = never,
	HookServices = never,
> =
	| ExcludeBuiltInWakeServices<R, Name, Database>
	| ExcludeBuiltInWakeServices<RX, Name, Database>
	| ExcludeBuiltInActionServices<
			ActionHandlerServices<ActionHandlers>,
			Name,
			Database
	  >
	| UnknownToNever<StateOptions.Services<State>>
	| UnknownToNever<Action.ServicesServer<Actions>>
	| UnknownToNever<Action.ServicesClient<Actions>>
	| UnknownToNever<ConnectionServicesServer<Connection>>
	| UnknownToNever<CreationInputDecodingServices<Input>>
	| ExcludeBuiltInPreWakeServices<RCreate, Name, Database>
	| ExcludeBuiltInPreWakeServices<RMigrate, Name, Database>
	| ExcludeBuiltInActionServices<HookServices, Name, Database>
	| Registry.Registry;

/**
 * A Rivet Actor contract. It carries the action schemas and
 * display options, but no server implementation.
 */
export interface Actor<
	Name extends string,
	Actions extends Action.Any = never,
	EventDefs extends Event.Any = never,
	Connection extends AnyConnectionContract = never,
	Input extends Schema.Top = never,
	Kind extends ContractKind = ContractKind,
> {
	readonly [TypeId]: typeof TypeId;
	readonly kind: Kind;
	readonly name: Name;
	readonly actions: ReadonlyArray<Actions>;
	/** The declared events. `events` is the server-side events API. */
	readonly eventDefs: ReadonlyArray<EventDefs>;
	readonly connection: Connection | undefined;
	readonly input: Input | undefined;

	of<ActionHandlers extends ActionHandlersFrom<Actions, Connection>>(
		actionHandlers: [Connection] extends [never] ? ActionHandlers : never,
	): ActionHandlers;

	/**
	 * `hooks` run while the actor is awake. An actor that declares
	 * `connection` must pass `authorize`.
	 *
	 * `Scope` is the scope of one request or connection for `http` and
	 * `webSocket`, which the SDK provides.
	 */
	of<
		ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
		RAuthorize = never,
		RConnect = never,
		RDisconnect = never,
		RHttp = never,
		RWebSocket = never,
		RActionResponse = never,
		RCanSubscribe = never,
		RDestroy = never,
	>(
		actionHandlers: ActionHandlers,
		hooks: Hooks<
			Actions,
			Connection,
			RAuthorize,
			RConnect,
			RDisconnect,
			RHttp,
			RWebSocket,
			RActionResponse,
			EventDefs,
			RCanSubscribe,
			RDestroy
		>,
	): Implementation<
		ActionHandlers,
		| RAuthorize
		| RActionResponse
		| RCanSubscribe
		| RDestroy
		| RConnect
		| RDisconnect
		| Exclude<RHttp, HttpServerRequest.HttpServerRequest | Scope.Scope>
		| Exclude<
				RWebSocket,
				| HttpServerRequest.HttpServerRequest
				| Socket.Socket
				| Scope.Scope
		  >
	>;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
		RCreate = never,
		RMigrate = never,
		HookServices = never,
		RBody = never,
		RBodyBuilder = never,
		ROnError = never,
		RFork = never,
	>(
		wake: ToLayerWake<
			Kind,
			Wake<
				HandlersOrImplementation<
					ActionHandlers,
					HookServices,
					Connection
				>,
				R,
				RX,
				WakeOptionsFor<never, Database>
			>,
			RBody,
			RBodyBuilder
		>,
		options: StatelessOptions<Database> &
			CreateHookOption<Input, RCreate> &
			MigrateHookOption<RMigrate> &
			ForkHookOption<Input, RFork> &
			WorkflowErrorHookOption<Kind, ROnError> &
			ActionsOption<
				Kind,
				Actions,
				Connection,
				Wake<
					HandlersOrImplementation<
						ActionHandlers,
						HookServices,
						Connection
					>,
					R,
					RX,
					WakeOptionsFor<never, Database>
				>
			>,
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<
			Name,
			Actions,
			ActionHandlers,
			never,
			Database,
			R,
			RX | RBodyBuilder,
			Connection,
			Input,
			RCreate | RFork,
			RMigrate,
			HookServices | ROnError | Exclude<RBody, Workflow.Workflow>
		>
	>;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
		R = never,
		RX = never,
		HookServices = never,
		RBody = never,
		RBodyBuilder = never,
	>(
		wake: ToLayerWake<
			Kind,
			Wake<
				HandlersOrImplementation<
					ActionHandlers,
					HookServices,
					Connection
				>,
				R,
				RX,
				WakeOptionsFor<never, undefined>
			>,
			RBody,
			RBodyBuilder
		>,
		...options: RequiredOptionsArgs<
			Kind,
			Actions,
			Connection,
			Wake<
				HandlersOrImplementation<
					ActionHandlers,
					HookServices,
					Connection
				>,
				R,
				RX,
				WakeOptionsFor<never, undefined>
			>
		>
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<
			Name,
			Actions,
			ActionHandlers,
			never,
			undefined,
			R,
			RX | RBodyBuilder,
			Connection,
			Input,
			never,
			never,
			HookServices | Exclude<RBody, Workflow.Workflow>
		>
	>;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
		State extends StateOptions.Any,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
		RCreate = never,
		RMigrate = never,
		HookServices = never,
		RBody = never,
		RBodyBuilder = never,
		ROnError = never,
		RFork = never,
	>(
		wake: ToLayerWake<
			Kind,
			Wake<
				HandlersOrImplementation<
					ActionHandlers,
					HookServices,
					Connection
				>,
				R,
				RX,
				WakeOptionsFor<State, Database>
			>,
			RBody,
			RBodyBuilder
		>,
		options: StatefulOptions<State, Database> &
			CreateHookOption<Input, RCreate> &
			MigrateHookOption<RMigrate> &
			ForkHookOption<Input, RFork> &
			WorkflowErrorHookOption<Kind, ROnError> &
			ActionsOption<
				Kind,
				Actions,
				Connection,
				Wake<
					HandlersOrImplementation<
						ActionHandlers,
						HookServices,
						Connection
					>,
					R,
					RX,
					WakeOptionsFor<State, Database>
				>
			>,
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<
			Name,
			Actions,
			ActionHandlers,
			State,
			Database,
			R,
			RX | RBodyBuilder,
			Connection,
			Input,
			RCreate | RFork,
			RMigrate,
			HookServices | ROnError | Exclude<RBody, Workflow.Workflow>
		>
	>;

	/**
	 * Effect-yielded typed accessor for this actor. Provide a
	 * `Client.layer({ ... })` once at the program root; every
	 * `yield* SomeActor.client` then dispatches through the same
	 * transport.
	 */
	readonly client: Effect.Effect<
		Accessor<Actions, EventDefs, Connection, Input>,
		never,
		Client.Client
	>;

	/**
	 * This actor's typed schedule. Use it in this actor's wake effect, its
	 * action handlers, or effects they run.
	 */
	readonly schedule: Effect.Effect<
		ActorSchedule<Actions>,
		never,
		Schedule<Name>
	>;

	/**
	 * This actor's typed recurring jobs. Use it in this actor's wake effect,
	 * its action handlers, or effects they run.
	 */
	readonly cron: Effect.Effect<
		ActorCronInternal.ActorCron<Actions>,
		never,
		Cron<Name>
	>;

	/**
	 * Effect-yielded events API typed to this actor's events. Yield it in the
	 * wake effect, in action handlers, or in effects they run.
	 */
	readonly events: Effect.Effect<ActorEvents<EventDefs>, never, Events<Name>>;

	/** This actor's open connections, typed to its `connection` params. */
	readonly connections: Effect.Effect<
		ActorConnections<Connection>,
		never,
		Connections<Name>
	>;

	/**
	 * This actor's SQLite database, as the `SqlClient` of `effect/sql`. Only
	 * an actor that passes `db` to `toLayer` has it. Use it in the wake
	 * effect, action handlers, hooks, `onCreate` and `onMigrate`.
	 *
	 * @example
	 * ```ts
	 * const CounterLive = Counter.toLayer(
	 *   Effect.gen(function* () {
	 *     const sql = yield* Counter.sql;
	 *
	 *     return Counter.of({
	 *       Increment: Effect.fn("Counter.Increment")(function* ({ payload }) {
	 *         const rows = yield* sql<{ count: number }>`
	 *           UPDATE counter SET count = count + ${payload.amount} RETURNING count
	 *         `;
	 *         return rows[0].count;
	 *       }),
	 *     });
	 *   }),
	 *   { db: db() },
	 * );
	 * ```
	 */
	readonly sql: Effect.Effect<SqlClient.SqlClient, never, Sql<Name>>;
}

export type Any = Actor<string, Action.AnyWithProps, Event.Any, any, any>;

export type ActionHandlersFrom<
	Actions extends Action.Any,
	Connection extends AnyConnectionContract = never,
> = {
	readonly [A in Actions as A["_tag"]]: (
		envelope: ActionRequest<A, ConnectionParams<Connection>>,
	) => Action.ResultFrom<A, any>;
};

type ActorWake<
	ActionHandlers,
	R,
	RX,
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = Wake<
	ActionHandlers | Implementation<ActionHandlers, any>,
	R,
	RX,
	WakeOptionsFor<State, Database>
>;

/** @internal */
export type WorkflowBodyBuilder = Effect.Effect<
	Effect.Effect<void, never, any>,
	never,
	any
>;

const Proto: Omit<
	Actor<any, any, any, any, any>,
	| "kind"
	| "name"
	| "actions"
	| "eventDefs"
	| "connection"
	| "input"
	| "schedule"
	| "cron"
	| "events"
	| "connections"
	| "sql"
> = {
	[TypeId]: TypeId,
	toLayer<
		Actions extends Action.AnyWithProps,
		Connection extends AnyConnectionContract,
		ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
		State extends StateOptions.Any = never,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
	>(
		this: Actor<string, Actions, any, Connection, any>,
		wake:
			| ActorWake<ActionHandlers, R, RX, State, Database>
			| WorkflowBodyBuilder,
		{
			actions = {} as ActionHandlers,
			...options
		}: Options<State, Database> & {
			readonly actions?: ActorWake<
				ActionHandlers,
				R,
				RX,
				State,
				Database
			>;
		} = {},
	) {
		const workflow =
			this.kind === "workflow"
				? { workflowBodyBuilder: wake as WorkflowBodyBuilder }
				: undefined;
		return RegistryService.pipe(
			Effect.flatMap((registry) =>
				registry
					.makeRivetkitActor({
						actor: this,
						...workflow,
						wakeHandler: toWakeHandler<
							| ActionHandlers
							| Implementation<ActionHandlers, any>,
							R,
							RX,
							WakeOptionsFor<State, Database>
						>(
							workflow
								? actions
								: (wake as ActorWake<
										ActionHandlers,
										R,
										RX,
										State,
										Database
									>),
						),
						options,
					})
					.pipe(
						Effect.tap((rivetKitActor) =>
							Effect.sync(() =>
								registry.rivetkitActors.set(
									this.name,
									rivetKitActor,
								),
							),
						),
					),
			),
			Layer.effectDiscard,
		);
	},
	get client() {
		return Client.Client.pipe(
			Effect.map((client) => client.makeActorAccessor(this as Any)),
		);
	},
	of: (actionHandlers: object, hooks?: AnyHooks): any =>
		hooks === undefined
			? actionHandlers
			: {
					[ImplementationTypeId]: { _HookServices: identity },
					actionHandlers,
					hooks,
				},
};

/** @internal */
export const makeContract =
	<Kind extends ContractKind>(kind: Kind) =>
	<
		const Name extends string,
		const Actions extends ReadonlyArray<Action.AnyWithProps> = readonly [],
		const EventDefs extends ReadonlyArray<Event.Any> = readonly [],
		Params extends Schema.Top = never,
		Error extends Schema.Top = Schema.Never,
		Input extends Schema.Top = never,
	>(
		name: Name,
		options?: {
			readonly actions?: Actions;
			readonly events?: EventDefs;
			/** Declares the input a client creates the actor with, which `onCreate` gets. */
			readonly input?: Input;
			/** Declares connection params, which `authorize` checks. */
			readonly connection?: {
				readonly params: Params;
				readonly error?: Error;
			};
		},
	): Actor<
		Name,
		Actions[number],
		EventDefs[number],
		[Params] extends [never] ? never : ConnectionContract<Params, Error>,
		Input,
		Kind
	> => {
		const self = Object.create(Proto);
		self.kind = kind;
		self.name = name;
		self.input = options?.input;
		self.actions = options?.actions ?? [];
		self.eventDefs = options?.events ?? [];
		self.schedule = Context.Service<
			Schedule<Name>,
			ActorSchedule<Actions[number]>
		>(`@rivetkit/effect/Actor/Schedule/${name}`);
		self.cron = Context.Service<
			Cron<Name>,
			ActorCronInternal.ActorCron<Actions[number]>
		>(`@rivetkit/effect/Actor/Cron/${name}`);
		self.events = Context.Service<
			Events<Name>,
			ActorEvents<EventDefs[number]>
		>(`@rivetkit/effect/Actor/Events/${name}`);
		self.connections = Context.Service<
			Connections<Name>,
			ActorConnections<AnyConnectionContract>
		>(`@rivetkit/effect/Actor/Connections/${name}`);
		self.sql = Context.Service<Sql<Name>, SqlClient.SqlClient>(
			`@rivetkit/effect/Actor/Sql/${name}`,
		);
		self.connection =
			options?.connection === undefined
				? undefined
				: {
						params: options.connection.params,
						error: options.connection.error ?? Schema.Never,
					};
		return self;
	};

/**
 * Define a Rivet Actor contract.
 */
export const make = makeContract("actor");

/**
 * Normalizes any supported actor wake declaration into a wake function.
 *
 * @remarks
 * {@link Actor.toLayer|`Actor.toLayer()`} accepts several equivalent shapes so actor
 * implementations can choose the amount of initialization they need:
 *
 * - a plain action-handler object
 * - an `Effect` that builds an action-handler object
 * - a wake function that receives `wakeOptions` and returns handlers
 * - a wake function that returns an `Effect` of handlers
 * - an `Effect` that builds one of those wake functions
 *
 * This helper resolves those forms lazily for each actor wake and returns a
 * single `(wakeOptions) => Effect<handlers>` representation. It does not run a
 * wake function until the returned function is invoked, so per-wake resources,
 * services, and state are scoped to the specific actor instance that is waking.
 *
 * @param wake - A supported wake declaration shape.
 * @returns A function that resolves action handlers for one actor wake.
 *
 * @internal
 */
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Effect.Effect<
		(wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>,
		never,
		RX
	>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R | RX>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Effect.Effect<(wakeOptions: W) => ActionHandlers, never, RX>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, RX>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	W extends WakeOptions = WakeOptions,
>(
	wake: (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	W extends WakeOptions = WakeOptions,
>(
	wake: (wakeOptions: W) => ActionHandlers,
): (wakeOptions: W) => Effect.Effect<ActionHandlers>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Effect.Effect<ActionHandlers, never, RX>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, RX>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	W extends WakeOptions = WakeOptions,
>(wake: ActionHandlers): (wakeOptions: W) => Effect.Effect<ActionHandlers>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Wake<ActionHandlers, R, RX, W>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R | RX>;
/** @internal */
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	RX,
	W extends WakeOptions = WakeOptions,
>(wake: Wake<ActionHandlers, R, RX, W>) {
	return (wakeOptions: W) => {
		const wakeEffect = Effect.isEffect(wake)
			? (wake as Effect.Effect<
					ActionHandlers | WakeFunction<ActionHandlers, R, W>,
					never,
					RX
				>)
			: Effect.succeed(wake);

		return wakeEffect.pipe(
			Effect.flatMap((resolvedWake) => {
				if (Predicate.isFunction(resolvedWake)) {
					const actionHandlers = resolvedWake(wakeOptions);
					return Effect.isEffect(actionHandlers)
						? actionHandlers
						: Effect.succeed(actionHandlers);
				}

				return Effect.succeed(resolvedWake);
			}),
		);
	};
}
