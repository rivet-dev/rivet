import { Context, Effect, Exit, type Fiber, FiberSet, Scope } from "effect";
import type * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type * as StateOptions from "./StateOptions.ts";
import type * as StateRuntime from "./StateRuntime.ts";

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
	readonly state?: StateRuntime.ActorState<StateDefinition>;
};

export const make = Effect.fnUntraced(function* <
	ActionHandlers,
	StateDefinition extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
	WakeOptions,
>({
	wakeHandler,
	stateRuntime,
	makeContext,
	makeWakeOptions,
}: {
	readonly wakeHandler: (
		wakeOptions: WakeOptions,
	) => Effect.Effect<ActionHandlers, never, any>;
	readonly stateRuntime: StateRuntime.Runtime<StateDefinition> | undefined;
	readonly makeContext: (
		c: WakeContext<StateDefinition, Database>,
		scope: Scope.Closeable,
	) => Context.Context<any>;
	readonly makeWakeOptions: (
		c: WakeContext<StateDefinition, Database>,
		state: StateRuntime.ActorState<StateDefinition> | undefined,
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
		const state = stateRuntime
			? yield* stateRuntime.makeStateView(c)
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
	});

	return {
		get: (actorId: string) => instances.get(actorId),
		onWake: async (c: WakeContext<StateDefinition, Database>) => {
			await runPromise(
				makeInstance(c).pipe(
					Effect.tap((instance) =>
						Effect.sync(() => {
							instances.set(c.actorId, instance);
						}),
					),
				),
			);
		},
		onStateChange: stateRuntime
			? (
					c: WakeContext<StateDefinition, Database>,
					newState: unknown,
				) => {
					const instance = instances.get(c.actorId);
					// State changes can arrive after teardown removes the instance.
					if (!instance) return;

					stateRuntime.publishChange(instance, newState);
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
