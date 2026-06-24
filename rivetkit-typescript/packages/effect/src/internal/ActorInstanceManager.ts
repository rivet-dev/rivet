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
