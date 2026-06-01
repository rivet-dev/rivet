import {
	Context,
	Effect,
	Exit,
	FiberSet,
	identity,
	Layer,
	Predicate,
	Record,
	Schema,
	Scope,
	Struct,
} from "effect";
import * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type * as Action from "./Action.ts";
import * as Client from "./Client.ts";
import * as ActionDispatcher from "./internal/ActionDispatcher.ts";
import * as StateRuntime from "./internal/StateRuntime.ts";
import type * as StateOptions from "./internal/StateOptions.ts";
import * as Registry from "./Registry.ts";
import type * as RivetError from "./RivetError.ts";
import type * as State from "./State.ts";

const TypeId = "~@rivetkit/effect/Actor";

export const isActor = (u: unknown): u is Actor<any, any> =>
	Predicate.hasProperty(u, TypeId);

const rivetkitActorOptionsKeys = [
	"name",
	"icon",
] as const satisfies ReadonlyArray<
	keyof NonNullable<Rivetkit.ActorOptionsInput>
>;

export type RivetkitActorOptions = Pick<
	NonNullable<Rivetkit.ActorOptionsInput>,
	(typeof rivetkitActorOptionsKeys)[number]
>;

/**
 * Per-actor instance options. Combines the public
 * `RivetkitActorOptions` (forwarded verbatim to `Rivetkit.actor`)
 * with the effect-SDK-only options.
 */
export type Options<
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
> = Readonly<RivetkitActorOptions> & {
	readonly state?: State;
	readonly db?: Database;
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
	readonly state: State;
	readonly db?: Database;
};

const splitOptions = <
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
>(
	options: Options<State, Database>,
) => ({
	rivetkitOptions: Struct.pick(options, rivetkitActorOptionsKeys),
	effectOptions: Struct.omit(options, rivetkitActorOptionsKeys),
});

/**
 * Per-instance identity carried inside the wake scope. An actor
 * instance is addressable in two ways:
 *
 * - `(name, key)` — stable user-facing pair (e.g. "Counter", ["counter-123"])
 * - `actorId` — opaque engine-assigned unique identifier
 *
 * Available inside `Actor.toLayer`'s wake effect via
 * `yield* Actor.CurrentAddress`.
 */
export type ActorAddress = Pick<
	Rivetkit.ActorContext<any, any, any, any, any, any, any, any>,
	"actorId" | "name" | "key"
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

export type ActionRequest<A extends Action.Any> =
	A extends Action.Action<
		infer Tag,
		infer Payload,
		infer _Success,
		infer _Error
	>
		? {
				readonly _tag: Tag;
				readonly action: A;
				readonly payload: Payload["Type"];
			}
		: never;

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
	? {}
	: {
			readonly state: State.State<
				StateOptions.Decoded<StateDefinition>,
				Schema.SchemaError
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
 * A typed handle for one actor instance. Each action becomes a
 * method that takes the action's payload-constructor input and
 * returns an Effect with the action's success / typed error
 * channels baked in.
 */
export type Handle<Actions extends Action.Any> = {
	readonly [A in Actions as Action.Tag<A>]: (
		payload: Action.PayloadConstructor<A>,
	) => Effect.Effect<
		Action.Success<A>,
		Action.Error<A> | RivetError.RivetError
	>;
};

/**
 * Yielded by `Actor.client`. Address an actor instance by key, then
 * dispatch typed action calls against the returned `Handle`.
 */
export type Accessor<Actions extends Action.Any> = {
	readonly getOrCreate: (key: AccessorKeyParam) => Handle<Actions>;
};

type UnknownToNever<T> = unknown extends T ? never : T;

type ExcludeBuiltInWakeServices<
	T,
	_State extends StateOptions.Any,
> = UnknownToNever<Exclude<T, Scope.Scope | CurrentAddress | Sleep>>;

type ToLayerRequirements<
	Actions extends Action.Any,
	ActionHandlers,
	State extends StateOptions.Any,
	R,
	RX,
> =
	| ExcludeBuiltInWakeServices<R, State>
	| ExcludeBuiltInWakeServices<RX, State>
	| UnknownToNever<ActionHandlerServices<ActionHandlers>>
	| UnknownToNever<Action.ServicesServer<Actions>>
	| UnknownToNever<Action.ServicesClient<Actions>>
	| Registry.Registry;

/**
 * A Rivet Actor contract. It carries the action schemas and
 * display options, but no server implementation.
 */
export interface Actor<
	Name extends string,
	Actions extends Action.Any = never,
> {
	readonly [TypeId]: typeof TypeId;
	readonly name: Name;
	readonly actions: ReadonlyArray<Actions>;

	of<ActionHandlers extends ActionHandlersFrom<Actions>>(
		actionHandlers: ActionHandlers,
	): ActionHandlers;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions>,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
	>(
		wake: Wake<ActionHandlers, R, RX, WakeOptionsFor<never, Database>>,
		options: StatelessOptions<Database>,
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<Actions, ActionHandlers, never, R, RX>
	>;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions>,
		R = never,
		RX = never,
	>(
		wake: Wake<ActionHandlers, R, RX, WakeOptionsFor<never, undefined>>,
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<Actions, ActionHandlers, never, R, RX>
	>;

	toLayer<
		ActionHandlers extends ActionHandlersFrom<Actions>,
		State extends StateOptions.Any,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
	>(
		wake: Wake<ActionHandlers, R, RX, WakeOptionsFor<State, Database>>,
		options: StatefulOptions<State, Database>,
	): Layer.Layer<
		never,
		never,
		ToLayerRequirements<Actions, ActionHandlers, State, R, RX>
	>;

	/**
	 * Effect-yielded typed accessor for this actor. Provide a
	 * `Client.layer({ ... })` once at the program root; every
	 * `yield* SomeActor.client` then dispatches through the same
	 * transport.
	 */
	readonly client: Effect.Effect<Accessor<Actions>, never, Client.Client>;
}

export type Any = Actor<string, Action.AnyWithProps>;

export type ActionHandlersFrom<Actions extends Action.Any> = {
	readonly [A in Actions as A["_tag"]]: (
		envelope: ActionRequest<A>,
	) => Action.ResultFrom<A, any>;
};

const Proto: Omit<Actor<any, any>, "name" | "actions"> = {
	[TypeId]: TypeId,
	toLayer<
		Actions extends Action.AnyWithProps,
		ActionHandlers extends ActionHandlersFrom<Actions>,
		State extends StateOptions.Any = never,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
	>(
		this: Actor<string, Actions>,
		wake: Wake<ActionHandlers, R, RX, WakeOptionsFor<State, Database>>,
		options: Options<State, Database> = {},
	) {
		return makeRivetkitActor({
			actor: this,
			wakeHandler: toWakeHandler<
				ActionHandlers,
				R,
				RX,
				WakeOptionsFor<State, Database>
			>(wake),
			options,
		}).pipe(
			Effect.flatMap((rivetKitActor) =>
				Registry.Registry.pipe(
					Effect.flatMap((registry) =>
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
	of: identity,
};

/**
 * Define a Rivet Actor contract.
 */
export const make = <
	const Name extends string,
	const Actions extends ReadonlyArray<Action.AnyWithProps> = readonly [],
>(
	name: Name,
	options?: {
		readonly actions?: Actions;
	},
): Actor<Name, Actions[number]> => {
	const self = Object.create(Proto);
	self.name = name;
	self.actions = options?.actions ?? [];
	return self;
};

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
export function toWakeHandler<
	ActionHandlers extends object,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Effect.Effect<(wakeOptions: W) => ActionHandlers, never, RX>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, RX>;
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	W extends WakeOptions = WakeOptions,
>(
	wake: (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R>;
export function toWakeHandler<
	ActionHandlers extends object,
	W extends WakeOptions = WakeOptions,
>(
	wake: (wakeOptions: W) => ActionHandlers,
): (wakeOptions: W) => Effect.Effect<ActionHandlers>;
export function toWakeHandler<
	ActionHandlers extends object,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Effect.Effect<ActionHandlers, never, RX>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, RX>;
export function toWakeHandler<
	ActionHandlers extends object,
	W extends WakeOptions = WakeOptions,
>(wake: ActionHandlers): (wakeOptions: W) => Effect.Effect<ActionHandlers>;
export function toWakeHandler<
	ActionHandlers extends object,
	R,
	RX,
	W extends WakeOptions = WakeOptions,
>(
	wake: Wake<ActionHandlers, R, RX, W>,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, R | RX>;
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
				if (typeof resolvedWake === "function") {
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

type ActorInstance<
	ActionHandlers,
	StateDefinition extends StateOptions.Any,
> = ActionDispatcher.Instance<ActionHandlers> &
	StateRuntime.Instance<StateDefinition> & {
	readonly scope: Scope.Closeable;
};

const makeRivetkitActor = Effect.fnUntraced(function* <
	Name extends string,
	Actions extends Action.AnyWithProps,
	ActionHandlers extends ActionHandlersFrom<Actions>,
	RX,
	State extends StateOptions.Any = never,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
>({
	actor,
	wakeHandler,
	options,
}: {
	readonly actor: Actor<Name, Actions>;
	readonly wakeHandler: (
		wakeOptions: WakeOptionsFor<State, Database>,
	) => Effect.Effect<ActionHandlers, never, RX>;
	readonly options: Options<State, Database>;
}) {
	// Snapshot the current Effect context so action callbacks
	// (which run in rivetkit’s plain Promise world) can run
	// handler effects against the same services the Registry.start /
	// Registry.test layer was provided with.
	const services = yield* Effect.context<any>();

	const { effectOptions, rivetkitOptions } = splitOptions(options);
	const stateRuntime =
		effectOptions.state === undefined
			? undefined
			: yield* StateRuntime.make<State>(effectOptions.state);

	const instances = new Map<string, ActorInstance<ActionHandlers, State>>();

	type RivetkitDefinition = RivetkitActorDefinitionFor<State, Database>;

	const makeInstance = Effect.fnUntraced(function* (
		c: Rivetkit.WakeContextOf<RivetkitDefinition>,
	): Effect.fn.Return<ActorInstance<ActionHandlers, State>, never, any> {
		const scope = yield* Scope.make();
		const state = stateRuntime
			? yield* stateRuntime.makeStateView(c)
			: undefined;

		const context = Context.mergeAll(
			Context.make(CurrentAddress, {
				actorId: c.actorId,
				name: c.name,
				key: c.key,
			}),
			Context.make(Scope.Scope, scope),
			Context.make(
				Sleep,
				Effect.sync(() => c.sleep()),
			),
		);
		const wakeOptions = {
			rawRivetkitContext: c,
			...(state ? { state } : {}),
		} as WakeOptionsFor<State, Database>;
		const actionHandlers = yield* wakeHandler(wakeOptions).pipe(
			Effect.provide(context),
		);
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

	const onWake = async (c: Rivetkit.WakeContextOf<RivetkitDefinition>) => {
		await Effect.runPromiseWith(services)(
			makeInstance(c).pipe(
				Effect.tap((instance) =>
					Effect.sync(() => {
						instances.set(c.actorId, instance);
					}),
				),
			),
		);
	};

	const actions = ActionDispatcher.make<
		Name,
		Actions,
		ActionHandlers,
		RivetkitDefinition
	>({
		actor,
		getInstance: (actorId) => instances.get(actorId),
	});

	const onStateChange = stateRuntime
		? (
				c: Rivetkit.WakeContextOf<RivetkitDefinition>,
				newState: unknown,
			) => {
				const instance = instances.get(c.actorId);
				// Late state-change callbacks can arrive after teardown removed the
				// instance. There is no live Effect state stream left to update.
				if (!instance) return;

				stateRuntime.publishChange(instance, newState);
			}
		: undefined;

	const cleanupInstance = Effect.fnUntraced(function* (actorId: string) {
		const instance = instances.get(actorId);
		// Actor teardown can be reported more than once across sleep
		// and destroy paths. Treat missing entries as already cleaned up.
		if (!instance) return;

		instances.delete(actorId);
		yield* Scope.close(instance.scope, Exit.void);
	});

	const onSleep = async (c: Rivetkit.SleepContextOf<RivetkitDefinition>) => {
		await Effect.runPromiseWith(services)(cleanupInstance(c.actorId));
	};

	const onDestroy = async (
		c: Rivetkit.DestroyContextOf<RivetkitDefinition>,
	) => {
		await Effect.runPromiseWith(services)(cleanupInstance(c.actorId));
	};

	return Rivetkit.actor<
		StateOptions.Encoded<State>,
		undefined,
		undefined,
		undefined,
		undefined,
		Database,
		Record<never, never>,
		Record<never, never>,
		any
	>({
		options: rivetkitOptions,
		...(effectOptions.db ? { db: effectOptions.db } : {}),
		onWake,
		...(stateRuntime
			? { createState: stateRuntime.createInitialState }
			: {}),
		actions,
		...(onStateChange ? { onStateChange } : {}),
		onSleep,
		onDestroy,
	});
});
