import { Context, Effect, Exit, type Fiber, FiberSet, Scope } from "effect";
import type * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type { ActionConnection } from "../Actor.ts";
import type * as ActionDispatcher from "./ActionDispatcher.ts";
import type * as ActorStateAdapter from "./ActorStateAdapter.ts";
import type * as ConnectionHooks from "./ConnectionHooks.ts";
import type * as RawHttpHandler from "./RawHttpHandler.ts";
import type * as RawWebSocketHandler from "./RawWebSocketHandler.ts";
import type * as StateOptions from "./StateOptions.ts";

export type RivetkitDefinitionFor<
	StateDefinition extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = Rivetkit.ActorDefinition<
	StateOptions.Encoded<StateDefinition>,
	undefined,
	undefined,
	undefined,
	undefined,
	Database,
	Record<never, never>,
	Record<never, never>,
	any
>;

type WakeContext<
	StateDefinition extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = Rivetkit.WakeContextOf<RivetkitDefinitionFor<StateDefinition, Database>>;

/** What one wake built: the action handlers, the hooks and the workflow body. */
export type WakeResult<ActionHandlers> = {
	readonly actionHandlers: ActionHandlers;
	/** The body of a workflow, which `WorkflowRun` runs for this wake. */
	readonly workflowBody: Effect.Effect<void, never, any> | undefined;
	readonly connectionHooks: ConnectionHooks.ConnectionHooks;
	readonly onBeforeActionResponse:
		| ActionDispatcher.ActionResponseHooks
		| undefined;
	readonly http: RawHttpHandler.RequestEffect | undefined;
	readonly webSocket: RawWebSocketHandler.WebSocketHandler<any> | undefined;
	/** The `onDestroy` hook, bound to the context of the wake effect. */
	readonly onDestroy: Effect.Effect<void> | undefined;
};

export type Instance<
	ActionHandlers,
	StateDefinition extends StateOptions.Any,
> = WakeResult<ActionHandlers> & {
	readonly runFork: <A, E>(
		effect: Effect.Effect<A, E, any>,
		options?: Effect.RunOptions,
	) => Fiber.Fiber<A, E>;
	readonly scope: Scope.Closeable;
	/** The layer's services and the services of this wake. */
	readonly context: Context.Context<any>;
	readonly state?: ActorStateAdapter.ActorState<StateDefinition>;
	/** Decodes the connection's params and keeps them until it closes. */
	readonly openConnection: (
		conn: RivetkitConn,
	) => Effect.Effect<ActionConnection<unknown>, Rivetkit.RivetError>;
	readonly connection: (
		conn: RivetkitConn,
	) => Effect.Effect<ActionConnection<unknown>, Rivetkit.RivetError>;
	readonly closeConnection: (connectionId: string) => Effect.Effect<void>;
	readonly isConnected: (connectionId: string) => boolean;
	/** Runs `onDisconnect` at most once per connection, then closes it. */
	readonly disconnectConnection: (
		conn: RivetkitConn,
	) => Effect.Effect<void, Rivetkit.RivetError>;
};

export type RivetkitConn = {
	readonly id: string;
	readonly params: unknown;
	readonly isHibernatable?: boolean;
};

export type TeardownReason = "sleep" | "destroy";

export type DecodeConnectionParams = (
	params: unknown,
) => Effect.Effect<unknown, Rivetkit.RivetError>;

/**
 * Tracks one scope per connection, created when a handler first reads it,
 * and the connection's decoded params. Each scope is a child of the wake
 * scope, so sleep and destroy also close it.
 */
const makeConnections = (
	wakeScope: Scope.Scope,
	isConnected: (connectionId: string) => boolean,
	decodeParams: DecodeConnectionParams | undefined,
	getOnDisconnect: () => ConnectionHooks.ConnectionHooks["onDisconnect"],
	restoredConns: Iterable<RivetkitConn>,
) => {
	const scopes = new Map<string, Scope.Closeable>();
	const paramsById = new Map<string, unknown>();
	/** Connections that are open and have not run `onDisconnect`. */
	const awaitingDisconnect = new Map<string, RivetkitConn>();
	for (const conn of restoredConns) awaitingDisconnect.set(conn.id, conn);

	const connectionScope = (connectionId: string): Scope.Scope => {
		const existing = scopes.get(connectionId);
		if (existing) return existing;

		const scope = Scope.forkUnsafe(wakeScope);
		// Core removes a connection before it reports the disconnect. So a
		// connection that is still present will get a later close, and one
		// that is gone gets a closed scope, which runs finalizers immediately.
		if (isConnected(connectionId)) {
			scopes.set(connectionId, scope);
		} else {
			Scope.closeUnsafe(scope, Exit.void);
		}
		return scope;
	};

	const makeConnection = (
		id: string,
		params: { readonly params: unknown } | undefined,
	): ActionConnection<unknown> =>
		({
			id,
			...params,
			/**
			 * Most handlers never read this, and the first read checks the
			 * connection list, which costs O(connections).
			 */
			get scope() {
				return connectionScope(id);
			},
		}) as ActionConnection<unknown>;

	const resolveConnection = (
		conn: RivetkitConn,
		shouldCache: () => boolean,
	) =>
		Effect.suspend(() => {
			if (!decodeParams) {
				return Effect.succeed(makeConnection(conn.id, undefined));
			}
			if (paramsById.has(conn.id)) {
				return Effect.succeed(
					makeConnection(conn.id, {
						params: paramsById.get(conn.id),
					}),
				);
			}
			return decodeParams(conn.params).pipe(
				Effect.map((params) => {
					if (shouldCache()) paramsById.set(conn.id, params);
					return makeConnection(conn.id, { params });
				}),
			);
		});

	const closeConnection = (connectionId: string) =>
		Effect.suspend(() => {
			awaitingDisconnect.delete(connectionId);
			paramsById.delete(connectionId);
			const scope = scopes.get(connectionId);
			if (!scope) return Effect.void;

			scopes.delete(connectionId);
			return Scope.close(scope, Exit.void);
		});

	/**
	 * RivetKit can report one disconnect twice, and teardown runs the hook
	 * for connections whose disconnect has not arrived. Uninterruptible, so
	 * teardown waits for a running hook when it closes the wake scope.
	 */
	const disconnectConnection = (conn: RivetkitConn) =>
		Effect.suspend(() => {
			const onDisconnect = getOnDisconnect();
			return (
				awaitingDisconnect.delete(conn.id) && onDisconnect !== undefined
					? resolveConnection(conn, () => false).pipe(
							Effect.flatMap(onDisconnect),
						)
					: Effect.void
			).pipe(Effect.ensuring(closeConnection(conn.id)));
		}).pipe(Effect.uninterruptible);

	return {
		openConnection: (conn: RivetkitConn) =>
			Effect.suspend(() => {
				awaitingDisconnect.set(conn.id, conn);
				return resolveConnection(conn, () => true);
			}),
		/**
		 * Connections restored after sleep never open again, so their params
		 * are decoded on first use.
		 */
		connection: (conn: RivetkitConn) =>
			resolveConnection(conn, () => isConnected(conn.id)),
		closeConnection,
		isConnected,
		disconnectConnection,
		/** A hibernatable connection stays open while the actor sleeps. */
		disconnectOpenConnections: (reason: TeardownReason) =>
			Effect.suspend(() =>
				Effect.forEach(
					Array.from(awaitingDisconnect.values()).filter(
						(conn) =>
							reason === "destroy" ||
							!conn.isHibernatable ||
							!isConnected(conn.id),
					),
					(conn) => Effect.exit(disconnectConnection(conn)),
					{ concurrency: "unbounded", discard: true },
				),
			),
	};
};

export const make = Effect.fnUntraced(function* <
	ActionHandlers,
	StateDefinition extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
	WakeOptions,
>({
	wakeHandler,
	stateAdapter,
	decodeConnectionParams,
	makeContext,
	makeWakeOptions,
}: {
	readonly wakeHandler: (
		wakeOptions: WakeOptions,
	) => Effect.Effect<WakeResult<ActionHandlers>, never, any>;
	readonly stateAdapter:
		| ActorStateAdapter.Adapter<StateDefinition>
		| undefined;
	readonly decodeConnectionParams: DecodeConnectionParams | undefined;
	readonly makeContext: (
		c: WakeContext<StateDefinition, Database>,
		scope: Scope.Closeable,
		resolveConnection: Instance<
			ActionHandlers,
			StateDefinition
		>["connection"],
	) => Effect.Effect<Context.Context<any>>;
	readonly makeWakeOptions: (
		c: WakeContext<StateDefinition, Database>,
		state: ActorStateAdapter.ActorState<StateDefinition> | undefined,
	) => WakeOptions;
}) {
	type ManagedInstance = Instance<ActionHandlers, StateDefinition> & {
		readonly disconnectOpenConnections: (
			reason: TeardownReason,
		) => Effect.Effect<void>;
	};
	const instances = new Map<string, ManagedInstance>();

	const services = yield* Effect.context<any>();
	const runPromise = Effect.runPromiseWith(services);

	const makeInstance = Effect.fnUntraced(function* (
		c: WakeContext<StateDefinition, Database>,
	): Effect.fn.Return<ManagedInstance, never, any> {
		const scope = yield* Scope.make();
		return yield* Effect.gen(function* () {
			const state = stateAdapter
				? yield* stateAdapter
						.makeStateView(c)
						.pipe(Effect.provideService(Scope.Scope, scope))
				: undefined;
			let wakeResult: WakeResult<ActionHandlers> | undefined;
			const connections = makeConnections(
				scope,
				(connectionId) => c.conns.has(connectionId),
				decodeConnectionParams,
				() => wakeResult?.connectionHooks.onDisconnect,
				c.conns.values(),
			);
			const context = yield* makeContext(
				c,
				scope,
				connections.connection,
			);
			wakeResult = yield* wakeHandler(makeWakeOptions(c, state)).pipe(
				Effect.provide(context),
			);
			const instanceContext = Context.merge(services, context);
			const runFork = yield* FiberSet.makeRuntime<
				any,
				unknown,
				unknown
			>().pipe(Effect.provide(instanceContext));

			return {
				...wakeResult,
				runFork,
				scope,
				context: instanceContext,
				state,
				...connections,
			};
		}).pipe(
			Effect.onError((cause) =>
				Scope.close(scope, Exit.failCause(cause)),
			),
		);
	});

	return {
		get: (actorId: string) => instances.get(actorId),
		onWake: async (c: WakeContext<StateDefinition, Database>) => {
			instances.set(c.actorId, await runPromise(makeInstance(c)));
		},
		onStateChange: stateAdapter
			? (
					c: WakeContext<StateDefinition, Database>,
					newState: unknown,
				) => {
					const instance = instances.get(c.actorId);
					// State changes can arrive after teardown removes the instance.
					if (!instance) return;

					stateAdapter.publishChange(instance, newState);
				}
			: undefined,
		/**
		 * RivetKit reports disconnects concurrently with sleep and destroy, so
		 * teardown runs `onDisconnect` for the connections it closes before
		 * the wake scope closes. A destroy runs `onDestroy` between them. Like
		 * RivetKit, a failed `onDestroy` is logged and the teardown goes on.
		 */
		onTeardown: async (
			c: { readonly actorId: string },
			reason: TeardownReason,
		) => {
			return runPromise(
				Effect.gen(function* () {
					const instance = instances.get(c.actorId);
					// Teardown can be reported through multiple lifecycle callbacks.
					if (!instance) return;

					instances.delete(c.actorId);
					const { onDestroy } = instance;
					yield* instance
						.disconnectOpenConnections(reason)
						.pipe(
							Effect.andThen(
								reason === "destroy" && onDestroy !== undefined
									? onDestroy.pipe(
											Effect.catchCause((cause) =>
												Effect.logError(
													"onDestroy failed, the actor is destroyed anyway",
													cause,
												),
											),
										)
									: Effect.void,
							),
							Effect.ensuring(
								Scope.close(instance.scope, Exit.void),
							),
						);
				}),
			);
		},
	};
});
