import { workflow as workflowsPackageWorkflow } from "@rivet-dev/workflows";
import {
	Array as Arr,
	Cause,
	Context,
	DateTime,
	Duration,
	Effect,
	Option,
	Record,
	Schema,
	Scope,
	Struct,
} from "effect";
import { SqlClient } from "effect/sql";
import * as Rivetkit from "rivetkit";
import type * as RivetkitDb from "rivetkit/db";
import type * as Action from "../Action.ts";
import {
	type ActionConnection,
	type ActionHandlersFrom,
	type Actor,
	type ActorConnections,
	type ActorEvents,
	type ActorSchedule,
	type AnyConnectionContract,
	type Connections,
	type Cron,
	CurrentAddress,
	Destroy,
	type Events,
	type Implementation,
	isImplementation,
	KeepAwake,
	type RivetkitActorOptions,
	type Schedule,
	ScheduledCallDecodeError,
	Sleep,
	type Sql,
	type WakeOptions,
	type WorkflowBodyBuilder,
} from "../Actor.ts";
import type * as Event from "../Event.ts";
import * as RivetError from "../RivetError.ts";
import * as ActionDispatcher from "./ActionDispatcher.ts";
import * as ActorCronInternal from "./ActorCron.ts";
import * as ActorInstanceManager from "./ActorInstanceManager.ts";
import * as ActorStateAdapter from "./ActorStateAdapter.ts";
import * as ConnectionHooks from "./ConnectionHooks.ts";
import * as LifecycleHooks from "./LifecycleHooks.ts";
import { makeActorLogAnnotations } from "./logging.ts";
import * as RawHttpHandler from "./RawHttpHandler.ts";
import * as RawWebSocketHandler from "./RawWebSocketHandler.ts";
import { makeSqlClient } from "./SqlClient.ts";
import type * as Workflow from "../Workflow.ts";
import type * as StateOptions from "./StateOptions.ts";
import * as WorkflowRun from "./WorkflowRun.ts";

type RivetkitActorOptionsInput = NonNullable<Rivetkit.ActorOptionsInput>;

const durationInputToMillis = (input: Duration.Input | undefined) =>
	input === undefined ? undefined : Duration.toMillis(input);

const rivetkitActorDurationOptionsToMillis = {
	sleepTimeout: durationInputToMillis,
	sleepGracePeriod: durationInputToMillis,
	actionTimeout: durationInputToMillis,
	onMigrateTimeout: durationInputToMillis,
	connectionLivenessTimeout: durationInputToMillis,
	connectionLivenessInterval: durationInputToMillis,
	onBeforeConnectTimeout: durationInputToMillis,
	onConnectTimeout: durationInputToMillis,
} as const satisfies Partial<
	Record<keyof RivetkitActorOptionsInput, typeof durationInputToMillis>
>;

export type RivetkitActorDurationOptionKey =
	keyof typeof rivetkitActorDurationOptionsToMillis;

const toRivetkitActorOptionsInput = (
	options: RivetkitActorOptions,
): RivetkitActorOptionsInput =>
	Struct.evolve(options, rivetkitActorDurationOptionsToMillis);

/**
 * Per-actor instance options. Combines the public
 * `RivetkitActorOptions` (forwarded to `Rivetkit.actor`)
 * with the effect-SDK-only options.
 */
export type Options<
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
> = Readonly<RivetkitActorOptions> & {
	/** @deprecated Actor state was deprecated in 3.0 and will be removed in a future version. Store durable data in SQLite (`c.db`) and in-memory data in `c.vars`. See https://rivet.dev/actors/docs/deprecated/state */
	readonly state?: State;
	readonly db?: Database;
	readonly onCreate?: (input: any) => Effect.Effect<void, never, any>;
	readonly onMigrate?: (options: {
		readonly isNew: boolean;
	}) => Effect.Effect<void, never, any>;
	readonly onFork?: (fork: any) => Effect.Effect<void, never, any>;
	readonly onError?: (
		event: Workflow.ErrorEvent,
	) => Effect.Effect<void, never, any>;
};

const effectActorOptionKeys = [
	"state",
	"db",
	"onCreate",
	"onMigrate",
	"onFork",
	"onError",
] as const satisfies ReadonlyArray<keyof Options<any, any>>;

const splitOptions = <
	State extends StateOptions.Any,
	Database extends RivetkitDb.AnyDatabaseProvider,
>(
	options: Options<State, Database>,
) => ({
	rivetkitOptions: toRivetkitActorOptionsInput(
		Struct.omit(options, effectActorOptionKeys),
	),
	effectOptions: Struct.pick(options, effectActorOptionKeys),
});

type RivetkitSchedule =
	Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>["schedule"];

type RivetkitScheduledEvent = Awaited<
	ReturnType<RivetkitSchedule["list"]>
>[number];

/**
 * Builds the codecs for an actor's action payloads once, and returns a
 * function that binds them to one wake's RivetKit schedule.
 */
const makeActorScheduleFactory = <Actions extends Action.AnyWithProps>(
	actor: Pick<Actor<string, Actions, any>, "name" | "actions">,
) => {
	const encodePayloadByTag = Record.fromIterableWith(
		actor.actions,
		(action) => [
			action._tag,
			Schema.encodeEffect(Schema.toCodecJson(action.payloadSchema)),
		],
	);

	/**
	 * Reads the payload from the stored arguments the same way the action
	 * dispatcher does when the call fires.
	 */
	const decodeArgsByTag = Record.fromIterableWith(actor.actions, (action) => {
		const decodePayload = Schema.decodeUnknownEffect(
			Schema.toCodecJson(action.payloadSchema),
		);
		return [
			action._tag,
			(args: ReadonlyArray<unknown>) =>
				decodePayload(
					!action.hasPayload && args[0] === undefined
						? null
						: args[0],
				),
		];
	});

	const decodeScheduledCall = (event: RivetkitScheduledEvent) =>
		Option.match(Record.get(decodeArgsByTag, event.action), {
			onNone: () =>
				Effect.fail(
					new ScheduledCallDecodeError({
						id: event.id,
						action: event.action,
					}),
				),
			onSome: (decodeArgs) =>
				decodeArgs(event.args).pipe(
					Effect.map((payload) => ({
						id: event.id,
						_tag: event.action,
						payload,
						runAt: DateTime.makeUnsafe(event.runAt),
					})),
					Effect.mapError(
						(cause) =>
							new ScheduledCallDecodeError({
								id: event.id,
								action: event.action,
								cause,
							}),
					),
				),
		});

	return (rivetkitSchedule: RivetkitSchedule): ActorSchedule<Actions> => {
		/**
		 * These become the scheduled call's arguments. A no-payload action
		 * gets none, like a raw RivetKit call.
		 */
		const encodeScheduleArgs = (
			action: Action.AnyWithProps,
			payload: ReadonlyArray<unknown>,
		): Effect.Effect<ReadonlyArray<unknown>, never, any> => {
			if (!action.hasPayload) return Effect.succeed([]);

			return encodePayloadByTag[action._tag](payload[0]).pipe(
				Effect.map((encodedPayload) => [encodedPayload]),
				Effect.orDie,
			);
		};

		const after = (
			delay: Duration.Input,
			action: Action.AnyWithProps,
			...payload: ReadonlyArray<unknown>
		) =>
			encodeScheduleArgs(action, payload).pipe(
				Effect.flatMap((args) =>
					Effect.tryPromise({
						try: () =>
							rivetkitSchedule.after(
								Duration.toMillis(delay),
								action._tag,
								...args,
							),
						catch: RivetError.fromUnknown,
					}),
				),
			);

		const at = (
			at: DateTime.DateTime | Date,
			action: Action.AnyWithProps,
			...payload: ReadonlyArray<unknown>
		) =>
			encodeScheduleArgs(action, payload).pipe(
				Effect.flatMap((args) =>
					Effect.tryPromise({
						try: () =>
							rivetkitSchedule.at(
								DateTime.toEpochMillis(DateTime.makeUnsafe(at)),
								action._tag,
								...args,
							),
						catch: RivetError.fromUnknown,
					}),
				),
			);

		const cancel = (id: string) =>
			Effect.tryPromise({
				try: () => rivetkitSchedule.cancel(id),
				catch: RivetError.fromUnknown,
			});

		const get = (id: string) =>
			Effect.tryPromise({
				try: () => rivetkitSchedule.get(id),
				catch: RivetError.fromUnknown,
			}).pipe(
				Effect.flatMap((event) =>
					event === undefined
						? Effect.succeedNone
						: Effect.asSome(decodeScheduledCall(event)),
				),
			);

		const list = Effect.tryPromise({
			try: () => rivetkitSchedule.list(),
			catch: RivetError.fromUnknown,
		}).pipe(
			Effect.flatMap((events) =>
				Effect.forEach(events, (event) =>
					decodeScheduledCall(event).pipe(
						Effect.asSome,
						Effect.catchTag("ScheduledCallDecodeError", (error) =>
							Effect.logWarning(
								"Skipped a scheduled call that does not decode",
								error,
							).pipe(
								Effect.annotateLogs({
									scheduleId: error.id,
									action: error.action,
								}),
								Effect.as(Option.none()),
							),
						),
					),
				),
			),
			Effect.map(Arr.getSomes),
		);

		/**
		 * The public signatures tie each payload to its action. The
		 * implementation looks the codecs up by tag instead.
		 */
		return {
			after,
			at,
			cancel,
			get,
			list,
		} as unknown as ActorSchedule<Actions>;
	};
};

const encodeEventPayloadByEvent = new WeakMap<
	Event.Any,
	(payload: unknown) => Effect.Effect<unknown, Schema.SchemaError, any>
>();

const encodeEventArgs = (
	event: Event.Any,
	payload: ReadonlyArray<unknown>,
): Effect.Effect<ReadonlyArray<unknown>, never, any> => {
	if (!event.hasPayload) return Effect.succeed([]);

	let encodePayload = encodeEventPayloadByEvent.get(event);
	if (encodePayload === undefined) {
		encodePayload = Schema.encodeEffect(
			Schema.toCodecJson(event.payloadSchema),
		);
		encodeEventPayloadByEvent.set(event, encodePayload);
	}
	return encodePayload(payload[0]).pipe(
		Effect.map((encodedPayload) => [encodedPayload]),
		Effect.orDie,
	);
};

const makeActorEvents = (
	c: Pick<
		Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>,
		"broadcast" | "conns"
	>,
): ActorEvents<Event.Any> => {
	const broadcast = (event: Event.Any, ...payload: ReadonlyArray<unknown>) =>
		encodeEventArgs(event, payload).pipe(
			Effect.flatMap((args) =>
				Effect.sync(() => c.broadcast(event._tag, ...args)),
			),
		);

	const send = (
		connection: ActionConnection,
		event: Event.Any,
		...payload: ReadonlyArray<unknown>
	) =>
		encodeEventArgs(event, payload).pipe(
			Effect.flatMap((args) =>
				Effect.sync(() =>
					c.conns.get(connection.id)?.send(event._tag, ...args),
				),
			),
		);

	/**
	 * The public signatures tie each payload to its event. The
	 * implementation encodes with the event's own schema instead.
	 */
	return { broadcast, send } as unknown as ActorEvents<Event.Any>;
};

/**
 * Builds the RivetKit actor definition of an actor contract. `Actor.toLayer`
 * reaches it through the `Registry` service, so the contract does not import
 * server code.
 */
export const makeRivetkitActor = Effect.fnUntraced(function* <
	Name extends string,
	Actions extends Action.AnyWithProps,
	Connection extends AnyConnectionContract,
	ActionHandlers extends ActionHandlersFrom<Actions, Connection>,
	RX,
	W extends WakeOptions,
	State extends StateOptions.Any = never,
	Database extends RivetkitDb.AnyDatabaseProvider = undefined,
>({
	actor,
	workflowBodyBuilder,
	wakeHandler,
	options,
}: {
	readonly actor: Actor<Name, Actions, any, Connection, any>;
	/** Runs on each wake before `wakeHandler`. Only a workflow contract has it. */
	readonly workflowBodyBuilder?: WorkflowBodyBuilder;
	readonly wakeHandler: (
		wakeOptions: W,
	) => Effect.Effect<
		ActionHandlers | Implementation<ActionHandlers, any>,
		never,
		RX
	>;
	readonly options: Options<State, Database>;
}) {
	const { effectOptions, rivetkitOptions } = splitOptions(options);
	for (const key of Struct.keys(rivetkitActorDurationOptionsToMillis)) {
		const millis = rivetkitOptions[key];
		if (millis !== undefined && !Number.isFinite(millis)) {
			return yield* Effect.die(
				new Cause.IllegalArgumentError(
					`Actor "${actor.name}" option ${key} must be a finite duration, got ${millis}`,
				),
			);
		}
	}
	const stateAdapter =
		effectOptions.state === undefined
			? undefined
			: yield* ActorStateAdapter.make<State>(effectOptions.state);

	const makeActorSchedule = makeActorScheduleFactory(actor);
	const addSqlClient = <Services>(
		context: Context.Context<Services>,
		sqlClient: SqlClient.SqlClient,
	) =>
		context.pipe(
			Context.add(SqlClient.SqlClient, sqlClient),
			Context.add(
				actor.sql as Context.Key<Sql<Name>, SqlClient.SqlClient>,
				sqlClient,
			),
		);
	const makeActorCron = ActorCronInternal.makeActorCronFactory(actor);

	const instanceManager = yield* ActorInstanceManager.make<
		ActionHandlers,
		State,
		Database,
		W
	>({
		wakeHandler: (wakeOptions) =>
			Effect.all([
				workflowBodyBuilder ?? Effect.succeed(undefined),
				wakeHandler(wakeOptions),
			]).pipe(
				Effect.flatMap(([workflowBody, wakeResult]) => {
					const { actionHandlers, hooks } = isImplementation(
						wakeResult,
					)
						? wakeResult
						: { actionHandlers: wakeResult, hooks: {} };
					return Effect.map(
						Effect.all([
							ConnectionHooks.bindToWakeContext(
								actor.name,
								hooks,
							),
							Effect.context<never>(),
						]),
						([connectionHooks, wakeContext]) => ({
							actionHandlers: actionHandlers as ActionHandlers,
							workflowBody,
							connectionHooks,
							onBeforeActionResponse:
								hooks.onBeforeActionResponse,
							http:
								hooks.http &&
								RawHttpHandler.toRequestEffect(hooks.http),
							webSocket: hooks.webSocket,
							onDestroy: hooks.onDestroy?.pipe(
								Effect.withSpan(`${actor.name}/onDestroy`, {
									kind: "server",
								}),
								Effect.provideContext(wakeContext),
							) as Effect.Effect<void> | undefined,
						}),
					);
				}),
				Effect.annotateLogs(
					makeActorLogAnnotations(wakeOptions.rawRivetkitContext),
				),
			),
		stateAdapter,
		decodeConnectionParams:
			ConnectionHooks.makeDecodeConnectionParams(actor),
		makeContext: (c, scope, resolveConnection) => {
			const context = Context.mergeAll(
				Context.make(CurrentAddress, {
					actorId: c.actorId,
					name: c.name,
					key: c.key,
					region: c.region,
					namespace: c.namespace,
					forkChain: c.forkChain,
				}),
				Context.make(Scope.Scope, scope),
				Context.make(
					Sleep,
					Effect.sync(() => c.sleep()),
				),
				/**
				 * RivetKit stays awake until the promise settles, so settle it
				 * when the effect exits.
				 */
				Context.make(KeepAwake, (effect) =>
					Effect.acquireUseRelease(
						Effect.sync(() => {
							let settle!: () => void;
							c.keepAwake(
								new Promise<void>((resolve) => {
									settle = resolve;
								}),
							);
							return settle;
						}),
						() => effect,
						(settle) => Effect.sync(settle),
					),
				),
				Context.make(
					Destroy,
					Effect.sync(() => c.destroy()),
				),
				Context.make(
					actor.events as Context.Key<
						Events<Name>,
						ActorEvents<Event.Any>
					>,
					makeActorEvents(c),
				),
				Context.make(
					actor.schedule as Context.Key<
						Schedule<Name>,
						ActorSchedule<Actions>
					>,
					makeActorSchedule(c.schedule),
				),
				Context.make(
					actor.cron as Context.Key<
						Cron<Name>,
						ActorCronInternal.ActorCron<Actions>
					>,
					makeActorCron(c.cron),
				),
				Context.make(
					actor.connections as unknown as Context.Key<
						Connections<Name>,
						ActorConnections<AnyConnectionContract>
					>,
					ConnectionHooks.makeActorConnections(
						c.conns,
						resolveConnection,
					),
				),
			);
			if (effectOptions.db === undefined) return Effect.succeed(context);

			return makeSqlClient(c.db as RivetkitDb.RawAccess).pipe(
				Effect.map((sqlClient) => addSqlClient(context, sqlClient)),
			);
		},
		makeWakeOptions: (c, state) =>
			({
				rawRivetkitContext: c,
				...(state === undefined ? {} : { state }),
			}) as W,
	});

	const actions = ActionDispatcher.make<
		Name,
		Actions,
		ActionHandlers,
		ActorInstanceManager.RivetkitDefinitionFor<State, Database>
	>({
		actor,
		getInstance: instanceManager.get,
	});

	const connectionCallbacks = ConnectionHooks.makeRivetkitCallbacks({
		actor,
		getInstance: instanceManager.get,
	});

	const lifecycleCallbacks = yield* LifecycleHooks.makeRivetkitCallbacks({
		actor,
		makeContext: (
			c: Pick<
				Rivetkit.WakeContextOf<
					ActorInstanceManager.RivetkitDefinitionFor<State, Database>
				>,
				| "actorId"
				| "name"
				| "key"
				| "region"
				| "namespace"
				| "forkChain"
				| "db"
			>,
		) => {
			const context = Context.make(CurrentAddress, {
				actorId: c.actorId,
				name: c.name,
				key: c.key,
				region: c.region,
				namespace: c.namespace,
				forkChain: c.forkChain,
			});
			if (effectOptions.db === undefined) return Effect.succeed(context);

			return makeSqlClient(c.db as RivetkitDb.RawAccess).pipe(
				Effect.map((sqlClient) => addSqlClient(context, sqlClient)),
			);
		},
		onCreate: effectOptions.onCreate,
		onMigrate: effectOptions.onMigrate,
		onFork: effectOptions.onFork,
	});

	const workflowRun =
		actor.kind === "workflow"
			? WorkflowRun.makeWorkflowRun({
					getInstance: instanceManager.get,
					getSleepGracePeriodMillis: () =>
						rivetkitActor.config.options.sleepGracePeriod,
				})
			: undefined;
	const { onError } = effectOptions;
	/** The workflow engine logs a rejection and goes on. */
	const workflowOptions =
		onError === undefined
			? undefined
			: {
					onError: async (
						c: Parameters<typeof makeActorLogAnnotations>[0],
						event: Workflow.ErrorEvent,
					) => {
						const instance = instanceManager.get(c.actorId);
						if (instance === undefined) return;
						await ActionDispatcher.runInstanceEffect(
							instance.runFork,
							onError(event).pipe(
								Effect.withSpan(`${actor.name}/onError`, {
									kind: "server",
								}),
								Effect.annotateLogs(makeActorLogAnnotations(c)),
							),
						);
					},
				};
	const defineRivetkitActor: typeof Rivetkit.actor<
		StateOptions.Encoded<State>,
		undefined,
		undefined,
		undefined,
		undefined,
		Database,
		Record<never, never>,
		Record<never, never>,
		any
	> =
		workflowRun === undefined
			? Rivetkit.actor
			: (config) =>
					// Inferred type arguments fail with TS2589.
					workflowsPackageWorkflow<
						any,
						any,
						any,
						any,
						any,
						any,
						any,
						any,
						any
					>(
						{
							...config,
							run: workflowRun.run,
						},
						workflowOptions,
					);

	const rivetkitActor = defineRivetkitActor({
		options: rivetkitOptions,
		...(effectOptions.db ? { db: effectOptions.db } : {}),
		...(lifecycleCallbacks.onCreate
			? { onCreate: lifecycleCallbacks.onCreate }
			: {}),
		...(lifecycleCallbacks.onMigrate
			? { onMigrate: lifecycleCallbacks.onMigrate }
			: {}),
		...(lifecycleCallbacks.onFork
			? { onFork: lifecycleCallbacks.onFork }
			: {}),
		onWake: instanceManager.onWake,
		...(stateAdapter
			? { createState: stateAdapter.createInitialState }
			: {}),
		actions,
		...(instanceManager.onStateChange
			? { onStateChange: instanceManager.onStateChange }
			: {}),
		...(connectionCallbacks.onBeforeConnect
			? { onBeforeConnect: connectionCallbacks.onBeforeConnect }
			: {}),
		...(connectionCallbacks.events
			? { events: connectionCallbacks.events }
			: {}),
		onConnect: connectionCallbacks.onConnect,
		onDisconnect: connectionCallbacks.onDisconnect,
		onRequest: RawHttpHandler.makeRivetkitOnRequest({
			getInstance: instanceManager.get,
		}),
		onWebSocket: RawWebSocketHandler.makeRivetkitOnWebSocket({
			actorName: actor.name,
			getInstance: instanceManager.get,
		}),
		onSleep: (c) => instanceManager.onTeardown(c, "sleep"),
		onDestroy: (c) => {
			workflowRun?.interruptOnDestroy(c.actorId);
			return instanceManager.onTeardown(c, "destroy");
		},
	});
	return rivetkitActor;
});
