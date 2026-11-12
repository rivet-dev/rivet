import { Context, Effect, Exit, type Fiber, FiberSet, Scope } from "effect";
import type * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type * as ActorStateAdapter from "./ActorStateAdapter.ts";
import type * as StateOptions from "./StateOptions.ts";

type RivetkitDefinitionFor<
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

export type Instance<
	ActionHandlers,
	StateDefinition extends StateOptions.Any,
> = {
	readonly actionHandlers: ActionHandlers;
	readonly runFork: <A, E>(
		effect: Effect.Effect<A, E, any>,
		options?: Effect.RunOptions,
	) => Fiber.Fiber<A, E>;
	readonly scope: Scope.Closeable;
	readonly state?: ActorStateAdapter.ActorState<StateDefinition>;
	readonly connectionScope: (connectionId: string) => Scope.Scope;
	readonly closeConnectionScope: (
		connectionId: string,
	) => Effect.Effect<void>;
};

/**
 * Tracks one scope per connection, created when a handler first reads it.
 * Each scope is a child of the wake scope, so sleep and destroy also close it.
 */
const makeConnectionScopes = (
	wakeScope: Scope.Scope,
	isConnected: (connectionId: string) => boolean,
) => {
	const scopes = new Map<string, Scope.Closeable>();

	return {
		connectionScope: (connectionId: string): Scope.Scope => {
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
		},
		closeConnectionScope: (connectionId: string) =>
			Effect.suspend(() => {
				const scope = scopes.get(connectionId);
				if (!scope) return Effect.void;

				scopes.delete(connectionId);
				return Scope.close(scope, Exit.void);
			}),
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
	makeContext,
	makeWakeOptions,
}: {
	readonly wakeHandler: (
		wakeOptions: WakeOptions,
	) => Effect.Effect<ActionHandlers, never, any>;
	readonly stateAdapter:
		| ActorStateAdapter.Adapter<StateDefinition>
		| undefined;
	readonly makeContext: (
		c: WakeContext<StateDefinition, Database>,
		scope: Scope.Closeable,
	) => Context.Context<any>;
	readonly makeWakeOptions: (
		c: WakeContext<StateDefinition, Database>,
		state: ActorStateAdapter.ActorState<StateDefinition> | undefined,
	) => WakeOptions;
}) {
	const instances = new Map<
		string,
		Instance<ActionHandlers, StateDefinition>
	>();

	const services = yield* Effect.context<any>();
	const runPromise = Effect.runPromiseWith(services);

	const makeInstance = Effect.fnUntraced(function* (
		c: WakeContext<StateDefinition, Database>,
	): Effect.fn.Return<Instance<ActionHandlers, StateDefinition>, never, any> {
		const scope = yield* Scope.make();
		return yield* Effect.gen(function* () {
			const state = stateAdapter
				? yield* stateAdapter
						.makeStateView(c)
						.pipe(Effect.provideService(Scope.Scope, scope))
				: undefined;
			const context = makeContext(c, scope);
			const actionHandlers = yield* wakeHandler(
				makeWakeOptions(c, state),
			).pipe(Effect.provide(context));
			const runFork = yield* FiberSet.makeRuntime<
				any,
				unknown,
				unknown
			>().pipe(Effect.provide(Context.merge(services, context)));

			return {
				actionHandlers,
				runFork,
				scope,
				state,
				...makeConnectionScopes(scope, (connectionId) =>
					c.conns.has(connectionId),
				),
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
		onDisconnect: async (
			c: { readonly actorId: string },
			conn: { readonly id: string },
		) => {
			const instance = instances.get(c.actorId);
			// The disconnect can arrive after teardown closed the wake scope.
			if (!instance) return;

			await runPromise(instance.closeConnectionScope(conn.id));
		},
		onTeardown: async (c: { readonly actorId: string }) => {
			return runPromise(
				Effect.gen(function* () {
					const instance = instances.get(c.actorId);
					// Teardown can be reported through multiple lifecycle callbacks.
					if (!instance) return;

					instances.delete(c.actorId);
					yield* Scope.close(instance.scope, Exit.void);
				}),
			);
		},
	};
});
