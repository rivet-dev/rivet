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
export type Options<State extends ActorState.AnyWithProps> =
	Readonly<RivetkitActorOptions> & {
		readonly state?: State;
		readonly db?: RivetkitDb.AnyDatabaseProvider;
	};

const splitOptions = <State extends ActorState.AnyWithProps>(
	options: Options<State>,
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

export class RawRivetkitContext extends Context.Service<
	RawRivetkitContext,
	Rivetkit.RunContextOf<Rivetkit.AnyActorDefinition>
>()("@rivetkit/effect/Actor/RawRivetkitContext") {}

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
		R,
		ActionHandlers extends ActionHandlersFrom<Actions>,
		State extends ActorState.AnyWithProps = never,
		RX = never,
	>(
		wake:
			| ActionHandlers
			| Effect.Effect<ActionHandlers, never, RX>
			| ((wakeOptions: any) => ActionHandlers)
			| ((wakeOptions: any) => Effect.Effect<ActionHandlers, never, R>)
			| Effect.Effect<
					(
						wakeOptions: any,
					) => Effect.Effect<ActionHandlers, never, R>,
					never,
					RX
			  >,
		options?: Options<State>,
	): Layer.Layer<
		never,
		never,
		| Exclude<
				RX,
				| Scope.Scope
				| CurrentAddress
				| Sleep
				| RawRivetkitContext
				| State
		  >
		| R
		| ActionHandlerServices<ActionHandlers>
		| Action.ServicesServer<Actions>
		| Action.ServicesClient<Actions>
		| Registry.Registry
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
		R,
		Actions extends Action.AnyWithProps,
		ActionHandlers extends ActionHandlersFrom<Actions>,
		State extends ActorState.AnyWithProps = never,
		RX = never,
	>(
		this: Actor<string, Actions>,
		wake:
			| ActionHandlers
			| Effect.Effect<ActionHandlers, never, RX>
			| ((wakeOptions: any) => ActionHandlers)
			| ((wakeOptions: any) => Effect.Effect<ActionHandlers, never, R>)
			| Effect.Effect<
					(
						wakeOptions: any,
					) => Effect.Effect<ActionHandlers, never, R>,
					never,
					RX
			  >,
		options: Options<State> = {},
	) {
		const wakeHandler: (
			wakeContext: Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>,
		) => Effect.Effect<ActionHandlers, never, R | RX> = Effect.isEffect(
			wake,
		)
			? (c) =>
					(wake as Effect.Effect<any, never, RX>).pipe(
						Effect.flatMap((resolved: any) =>
							typeof resolved === "function"
								? (resolved(c) as Effect.Effect<
										ActionHandlers,
										never,
										R
									>)
								: Effect.succeed(resolved as ActionHandlers),
						),
					)
			: typeof wake === "function"
				? (c: any) => {
						const result = (wake as Function)(c);
						return (
							Effect.isEffect(result)
								? result
								: Effect.succeed(result as ActionHandlers)
						) as Effect.Effect<ActionHandlers, never, R>;
					}
				: () => Effect.succeed(wake as ActionHandlers);

		return makeRivetkitActor({
			actor: this,
			wakeHandler,
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

const makeRivetkitActor = Effect.fnUntraced(function* <
	Name extends string,
	Actions extends Action.AnyWithProps,
	ActionHandlers extends ActionHandlersFrom<Actions>,
	RX,
	State extends ActorState.AnyWithProps = never,
>({
	actor,
	wakeHandler,
	options,
}: {
	readonly actor: Actor<Name, Actions>;
	readonly wakeHandler: (
		wakeContext: Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>,
	) => Effect.Effect<ActionHandlers, never, RX>;
	readonly options: Options<State>;
}) {
	// Snapshot the current Effect context so action callbacks
	// (which run in rivetkit's plain Promise world) can run
	// handler effects against the same services the Registry.start /
	// Registry.test layer was provided with.
	const services = yield* Effect.context<any>();

	const { effectOptions, rivetkitOptions } = splitOptions(options);
	const stateCodec = UndefinedOr.map(effectOptions.state, (state) => ({
		decode: Schema.decodeUnknownEffect(state.schema),
		encode: Schema.encodeUnknownEffect(state.schema),
	}));

	const instances = MutableHashMap.empty<
		string,
		{
			readonly actionHandlers: ActionHandlers;
			readonly scope: Scope.Closeable;
			readonly state?: State.State<
				State["schema"]["Type"],
				Schema.SchemaError
			>;
		}
	>();

	const onWake = async (
		c: Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>,
	) => {
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
					Context.make(RawRivetkitContext, c),
					effectOptions.state
						? Context.make(
								effectOptions.state,
								UndefinedOr.getOrThrow(state),
							)
						: Context.empty(),
				);

				const actionHandlers = yield* wakeHandler(c).pipe(
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
				c: Rivetkit.ActionContextOf<Rivetkit.AnyActorDefinition>,
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
		c: Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>,
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
							.decode(newState)
							.pipe(Effect.orDie);
						State.publishUnsafe(state, decoded);
					}),
				);
			}),
		);
	};

	const onSleep = async (
		c: Rivetkit.SleepContextOf<Rivetkit.AnyActorDefinition>,
	) => {
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

	return Rivetkit.actor({
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
