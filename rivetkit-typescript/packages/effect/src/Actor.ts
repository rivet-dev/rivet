import {
	Cause,
	Context,
	Effect,
	Exit,
	identity,
	Layer,
	MutableHashMap,
	Option,
	Predicate,
	Record,
	Schema,
	Scope,
	Semaphore,
	Struct,
	Tracer,
	UndefinedOr,
} from "effect";
import * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type * as Action from "./Action";
import type * as ActorState from "./ActorState";
import * as Client from "./Client";
import * as ActionError from "./internal/ActionError";
import { readTraceMeta, rpcSystem } from "./internal/tracing";
import * as Registry from "./Registry";
import type * as RivetError from "./RivetError";
import * as State from "./State";

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
	State extends ActorState.AnyWithProps,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
> = Readonly<RivetkitActorOptions> & {
	readonly state?: State;
	readonly db?: Database;
};

const splitOptions = <
	State extends ActorState.AnyWithProps,
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

type ActorStateEncoded<State extends ActorState.AnyWithProps> =
	| State["schema"]["Encoded"]
	| ([State] extends [never] ? undefined : never);

type ActorStateDecoded<State extends ActorState.AnyWithProps> =
	State["schema"]["Type"];

type ActorStateCodec<State extends ActorState.AnyWithProps> = {
	readonly decode: (
		input: ActorStateEncoded<State>,
	) => Effect.Effect<
		ActorStateDecoded<State>,
		Schema.SchemaError,
		State["schema"]["DecodingServices"]
	>;
	readonly decodeUnknown: (
		input: unknown,
	) => Effect.Effect<
		ActorStateDecoded<State>,
		Schema.SchemaError,
		State["schema"]["DecodingServices"]
	>;
	readonly encode: (
		input: ActorStateDecoded<State>,
	) => Effect.Effect<
		ActorStateEncoded<State>,
		Schema.SchemaError,
		State["schema"]["EncodingServices"]
	>;
};

const makeActorStateCodec = <State extends ActorState.AnyWithProps>(
	state: State,
): ActorStateCodec<State> => {
	const schema = state.schema as State["schema"];

	return {
		decode: Schema.decodeEffect(schema),
		decodeUnknown: Schema.decodeUnknownEffect(schema),
		encode: Schema.encodeEffect(schema),
	};
};

type RivetkitActorDefinitionFor<
	State extends ActorState.AnyWithProps,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = Rivetkit.ActorDefinition<
	ActorStateEncoded<State>,
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
	State extends ActorState.AnyWithProps,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = {
	[Key in keyof Rivetkit.WakeContextOf<
		RivetkitActorDefinitionFor<State, Database>
	>]: Key extends "state"
		? [State] extends [never]
			? never
			: ActorStateEncoded<State>
		: Rivetkit.WakeContextOf<
				RivetkitActorDefinitionFor<State, Database>
			>[Key];
};

type WakeOptionsFor<
	ActorStateDefinition extends ActorState.AnyWithProps,
	Database extends RivetkitDb.AnyDatabaseProvider,
> = {
	readonly rawRivetkitContext: RawWakeContextFor<
		ActorStateDefinition,
		Database
	>;
} &
	([ActorStateDefinition] extends [never]
		? {}
		: {
				readonly state: State.State<
					ActorStateDecoded<ActorStateDefinition>,
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
	_State extends ActorState.AnyWithProps,
> = UnknownToNever<Exclude<T, Scope.Scope | CurrentAddress | Sleep>>;

type ToLayerRequirements<
	Actions extends Action.Any,
	ActionHandlers,
	State extends ActorState.AnyWithProps,
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
	in out Name extends string,
	in out Actions extends Action.Any = never,
> {
	readonly [TypeId]: typeof TypeId;
	readonly name: Name;
	readonly actions: ReadonlyArray<Actions>;

	of<ActionHandlers extends ActionHandlersFrom<Actions>>(
		actionHandlers: ActionHandlers,
	): ActionHandlers;

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
		State extends ActorState.AnyWithProps,
		Database extends RivetkitDb.AnyDatabaseProvider = undefined,
		R = never,
		RX = never,
	>(
		wake: Wake<ActionHandlers, R, RX, WakeOptionsFor<State, Database>>,
		options: Options<State, Database>,
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
	readonly [Action in Actions as Action["_tag"]]: (
		envelope: ActionRequest<Action>,
	) => Action.ResultFrom<Action, any>;
};

const Proto: Omit<Actor<any, any>, "name" | "actions"> = {
	[TypeId]: TypeId,
	toLayer<
		Actions extends Action.AnyWithProps,
		ActionHandlers extends ActionHandlersFrom<Actions>,
		State extends ActorState.AnyWithProps = never,
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
	self.actions = options?.actions;
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
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, never>;
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
>(
	wake: ActionHandlers,
): (wakeOptions: W) => Effect.Effect<ActionHandlers, never, never>;
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

const makeRivetkitActor = Effect.fnUntraced(function* <
	Name extends string,
	Actions extends Action.AnyWithProps,
	ActionHandlers extends ActionHandlersFrom<Actions>,
	RX,
	State extends ActorState.AnyWithProps = never,
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
	// (which run in rivetkit's plain Promise world) can run
	// handler effects against the same services the Registry.start /
	// Registry.test layer was provided with.
	const services = yield* Effect.context<any>();

	const { effectOptions, rivetkitOptions } = splitOptions(options);
	const stateCodec = UndefinedOr.map(
		effectOptions.state,
		makeActorStateCodec,
	);

	const instances = MutableHashMap.empty<
		string,
		{
			readonly actionHandlers: ActionHandlers;
			readonly scope: Scope.Closeable;
			readonly state?: State.State<
				ActorStateDecoded<State>,
				Schema.SchemaError
			>;
		}
	>();

	type RivetkitDefinition = RivetkitActorDefinitionFor<State, Database>;

	const onWake = async (c: Rivetkit.WakeContextOf<RivetkitDefinition>) => {
		await Effect.runPromiseWith(services)(
			Effect.gen(function* () {
				const scope = yield* Scope.make();

				const state = stateCodec
					? // `c.state` IS the state — `State` is just a typed
						// view + change stream over it. Effect-typed
						// read/write so async schema transforms work,
						// and `SchemaError` flows through `State.get` /
						// `set` / `update` to action handlers. The
						// wake-time initial read still dies if persisted
						// state can't be decoded — no caller exists yet
						// to handle it. `Schema.Top`'s requirements show
						// up as `unknown`; the captured `services`
						// context satisfies them at runtime, so we erase
						// R at the boundary.
						((yield* State.make(
							() => stateCodec.decode(c.state),
							(next) =>
								stateCodec.encode(next).pipe(
									Effect.tap((encoded) =>
										Effect.sync(() => {
											c.state = encoded;
										}),
									),
									Effect.asVoid,
								),
						).pipe(Effect.orDie)) as State.State<
							ActorState.AnyWithProps["schema"]["Type"],
							Schema.SchemaError
						>)
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

				yield* Effect.sync(() =>
					MutableHashMap.set(instances, c.actorId, {
						actionHandlers,
						scope,
						state,
					}),
				);
			}),
		);
	};

	const actions = Record.fromIterableWith(actor.actions, (action) => {
		const decodePayload = Schema.decodeUnknownEffect(
			Schema.toCodecJson(action.payloadSchema),
		);
		const encodeSuccess = Schema.encodeEffect(
			Schema.toCodecJson(action.successSchema),
		);
		const encodeError = Schema.encodeEffect(
			Schema.toCodecJson(action.errorSchema),
		);

		return [
			action._tag,
			async (
				c: Rivetkit.ActionContextOf<RivetkitDefinition>,
				payload: Action.Payload<typeof action>,
				meta?: Client.ActionMeta, // TODO: Find better type
			) => {
				// Always wrap in a server-side span so the handler has a
				// live `currentSpan` even when the caller didn't ship trace
				// context (e.g. a non-Effect-SDK client). When trace context
				// is present, reattach it as the parent so the server span
				// joins the caller's trace.
				const rpcMethod = `${actor.name}/${action._tag}`;
				const traceMeta = readTraceMeta(meta);

				const exit = await Effect.runPromiseExitWith(services)(
					Effect.gen(function* () {
						const instance = yield* MutableHashMap.get(
							instances,
							c.actorId,
						).pipe(Effect.fromOption, Effect.orDie);
						// The handler map is keyed by the same action
						// definitions being registered here, but
						// TypeScript loses that relationship once the
						// actions are widened into the RivetKit actions
						// record.
						const actionHandler = instance.actionHandlers[
							action._tag as keyof ActionHandlers
						] as (
							envelope: ActionRequest<typeof action>,
						) => Action.ResultFrom<typeof action, any>;
						const decodedPayload = yield* decodePayload(
							payload,
						).pipe(Effect.orDie);
						// The payload was decoded with this action's schema,
						// so this is the runtime boundary that restores the
						// typed envelope expected by the user handler.
						const actionRequest = {
							_tag: action._tag,
							action,
							payload: decodedPayload,
						} as ActionRequest<typeof action>;

						const resultExit = yield* Effect.exit(
							actionHandler(actionRequest),
						);

						if (Exit.isSuccess(resultExit)) {
							return yield* encodeSuccess(resultExit.value).pipe(
								Effect.orDie,
							);
						}

						const expectedError = Exit.findErrorOption(resultExit);

						if (Option.isSome(expectedError)) {
							const encodedError = yield* encodeError(
								expectedError.value,
							).pipe(Effect.orDie);

							return yield* Effect.fail(
								ActionError.make(action._tag, encodedError),
							);
						}

						// Defect / interruption. Do not encode these as action errors.
						// Let them escape, so Rivetkit maps them to its internal_error shape.
						return yield* Effect.die(
							Cause.squash(resultExit.cause),
						);
					}).pipe(
						Effect.withSpan(rpcMethod, {
							parent: traceMeta
								? Tracer.externalSpan(traceMeta)
								: undefined,
							kind: "server",
							attributes: {
								"rpc.system.name": rpcSystem,
								"rpc.method": rpcMethod,
							},
						}),
					),
				);

				if (Exit.isSuccess(exit)) return exit.value;
				throw Cause.squash(exit.cause);
			},
		];
	});

	const onStateChange = (
		c: Rivetkit.WakeContextOf<RivetkitDefinition>,
		newState: unknown,
	) => {
		void Effect.runForkWith(services)(
			Effect.gen(function* () {
				if (!stateCodec) return;

				const instance = yield* MutableHashMap.get(
					instances,
					c.actorId,
				).pipe(Effect.fromOption, Effect.orDie);

				const state = yield* Effect.fromNullishOr(instance.state).pipe(
					Effect.orDie,
				);

				yield* Semaphore.withPermit(
					state.semaphore,
					Effect.gen(function* () {
						const decoded = yield* stateCodec
							.decodeUnknown(newState)
							.pipe(Effect.orDie);
						State.publishUnsafe(state, decoded);
					}),
				);
			}),
		);
	};

	const onSleep = async (c: Rivetkit.SleepContextOf<RivetkitDefinition>) => {
		await Effect.runPromiseWith(services)(
			Effect.gen(function* () {
				const instance = yield* MutableHashMap.get(
					instances,
					c.actorId,
				).pipe(Effect.fromOption, Effect.orDie);
				yield* Scope.close(instance.scope, Exit.void);
				yield* Effect.sync(() => {
					MutableHashMap.remove(instances, c.actorId);
				});
			}),
		);
	};

	return Rivetkit.actor<
		ActorStateEncoded<State>,
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
		...(options.state
			? {
					createState: () =>
						Effect.runPromiseWith(services)(
							UndefinedOr.getOrThrow(stateCodec)
								.encode(
									UndefinedOr.getOrThrow(
										options.state,
									).initialValue(),
								)
								.pipe(Effect.orDie),
						),
				}
			: {}),
		actions,
		onStateChange,
		onSleep,
	});
});
