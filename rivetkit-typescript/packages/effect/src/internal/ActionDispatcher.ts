import {
	Cause,
	Effect,
	Exit,
	type Fiber,
	Option,
	Record,
	Schema,
	Tracer,
} from "effect";
import * as Rivetkit from "rivetkit";
import type * as Action from "../Action.ts";
import type {
	ActionConnection,
	ActionHandlersFrom,
	ActionRequest,
	Actor,
} from "../Actor.ts";
import * as ActionErrorEnvelope from "./ActionErrorEnvelope.ts";
import { makeActorLogAnnotations } from "./logging.ts";
import { readTraceMeta, rpcSystem } from "./tracing.ts";

type RunFork = <A, E>(
	effect: Effect.Effect<A, E, any>,
	options?: Effect.RunOptions,
) => Fiber.Fiber<A, E>;

/** The `onBeforeActionResponse` hooks as `MyActor.of` gets them, by action tag. */
export type ActionResponseHooks = {
	readonly [tag: string]:
		| ((response: any) => Effect.Effect<unknown, never, any>)
		| undefined;
};

export type Instance<ActionHandlers> = {
	readonly actionHandlers: ActionHandlers;
	readonly onBeforeActionResponse?: ActionResponseHooks | undefined;
	readonly runFork: RunFork;
	readonly connection: (conn: {
		readonly id: string;
		readonly params: unknown;
	}) => Effect.Effect<ActionConnection<unknown>, Rivetkit.RivetError>;
};

/**
 * Runs `effect` in the actor instance runtime and settles the promise the way
 * RivetKit expects from a callback.
 */
export const runInstanceEffect = async <A>(
	runFork: RunFork,
	effect: Effect.Effect<A, unknown, any>,
	options?: Effect.RunOptions,
): Promise<A> => {
	const fiber = runFork(effect, options);
	const exit = await new Promise<Exit.Exit<A, unknown>>((resolve) =>
		fiber.addObserver(resolve),
	);

	if (Exit.isSuccess(exit)) return exit.value;
	/**
	 * Fibers can be interrupted by a caller abort signal or by the actor
	 * instance scope closing during sleep, destroy, or shutdown. Surface those
	 * lifecycle exits as RivetKit's structured action-aborted error instead of
	 * an internal error.
	 */
	if (Cause.hasInterruptsOnly(exit.cause)) {
		throw makeActorAbortedError();
	}
	const expectedError = Exit.findErrorOption(exit);
	if (Option.isSome(expectedError)) {
		throw expectedError.value;
	}
	throw Cause.squash(exit.cause);
};

export const make = <
	Name extends string,
	Actions extends Action.AnyWithProps,
	ActionHandlers extends ActionHandlersFrom<Actions, any>,
	ActorDefinition extends Rivetkit.AnyActorDefinition,
>({
	actor,
	getInstance,
}: {
	readonly actor: Pick<Actor<Name, Actions, any>, "name" | "actions">;
	readonly getInstance: (
		actorId: string,
	) => Instance<ActionHandlers> | undefined;
}) =>
	Record.fromIterableWith(actor.actions, (action) => {
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
				c: Rivetkit.ActionContextOf<ActorDefinition>,
				...args: unknown[]
			) => {
				// Scheduler metadata is appended after stored arguments. Only strip
				// the trusted invocation object, never a caller-supplied lookalike.
				if (c.scheduledFire && args.at(-1) === c.scheduledFire) {
					args.pop();
				}
				const [payload, meta] = args;
				// Always wrap in a server-side span so the handler has a
				// live `currentSpan` even when the caller didn't ship trace
				// context (e.g., a non-Effect-SDK client). When trace context
				// is present, reattach it as the parent so the server span
				// joins the caller's trace.
				const rpcMethod = `${actor.name}/${action._tag}`;
				const traceMeta = readTraceMeta(meta);

				const instance = getInstance(c.actorId);
				/**
				 * Sleep and destroy can remove the instance before the request's
				 * abort signal fires, so a missing instance is a lifecycle abort.
				 */
				if (!instance) throw makeActorAbortedError();

				const actionEffect = Effect.gen(function* () {
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
					// Raw RivetKit clients call no-argument actions with an
					// absent first argument. The Effect JSON Void codec expects
					// null, so adapt only actions that declared no payload.
					const payloadForDecode =
						!action.hasPayload && payload === undefined
							? null
							: payload;
					const decodedPayload = yield* decodePayload(
						payloadForDecode,
					).pipe(
						Effect.tapError((schemaError) =>
							Effect.logDebug(
								`Invalid payload for action ${actor.name}/${action._tag}`,
								schemaError,
							),
						),
						Effect.mapError(
							() =>
								new Rivetkit.RivetError(
									"request",
									"invalid",
									`Invalid payload for action ${actor.name}/${action._tag}`,
								),
						),
					);
					/**
					 * Scheduled actions run without a connection, although the
					 * RivetKit context type declares `conn` as always present.
					 */
					const connection = c.conn
						? Option.some(yield* instance.connection(c.conn))
						: Option.none();
					// The payload was decoded with this action's schema,
					// so this is the runtime boundary that restores the
					// typed envelope expected by the user handler.
					const actionRequest = {
						_tag: action._tag,
						action,
						payload: decodedPayload,
						connection,
					} as unknown as ActionRequest<typeof action>;

					const resultExit = yield* Effect.exit(
						actionHandler(actionRequest),
					);

					if (Exit.isSuccess(resultExit)) {
						const onBeforeActionResponse =
							instance.onBeforeActionResponse?.[action._tag];
						const success =
							onBeforeActionResponse === undefined
								? resultExit.value
								: yield* onBeforeActionResponse({
										...actionRequest,
										success: resultExit.value,
									});
						return yield* encodeSuccess(success).pipe(Effect.orDie);
					}

					const expectedError = Exit.findErrorOption(resultExit);

					if (Option.isSome(expectedError)) {
						const encodedError = yield* encodeError(
							expectedError.value,
						).pipe(Effect.orDie);

						return yield* Effect.fail(
							ActionErrorEnvelope.toUserError(
								encodedError,
								`${action._tag} failed`,
							),
						);
					}

					return yield* Effect.failCause(resultExit.cause);
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
					Effect.annotateLogs(makeActorLogAnnotations(c)),
				);
				return runInstanceEffect(instance.runFork, actionEffect, {
					signal: c.abortSignal,
				});
			},
		];
	});

export const makeActorAbortedError = () =>
	new Rivetkit.RivetError("actor", "aborted", "Actor aborted", {
		public: true,
	});
