import {
	Cause,
	Context,
	Effect,
	Layer,
	Queue,
	Record,
	Result,
	Schema,
	Stream,
} from "effect";
import { Socket } from "effect/socket";
import * as RivetkitClient from "rivetkit/client";
import * as RivetkitErrors from "rivetkit/errors";
import * as RivetkitLog from "rivetkit/log";
import type * as Action from "./Action.ts";
import type * as Actor from "./Actor.ts";
import type * as Event from "./Event.ts";
import * as ActionErrorEnvelope from "./internal/ActionErrorEnvelope.ts";
import { getOrCreateBaseLogger } from "./internal/logging.ts";
import { rpcSystem, type TraceMeta } from "./internal/tracing.ts";
import * as RivetError from "./RivetError.ts";
import * as RivetLogger from "./RivetLogger.ts";

const TypeId = "~@rivetkit/effect/Client";

/**
 * Connection options for the Rivet Engine client transport. A subset of
 * rivetkit's `ClientConfigInput`.
 */
export type Options = Pick<
	RivetkitClient.ClientConfigInput,
	"endpoint" | "token" | "getToken" | "namespace" | "poolName" | "headers"
>;

/**
 * Per-call metadata envelope shipped as `args[1]` alongside the encoded
 * payload. The SDK currently uses it for trace propagation (`trace`),
 * but it's intentionally extensible so future cross-cutting concerns —
 * idempotency keys, deadlines, custom headers — can land as additional
 * optional fields without changing the wire shape.
 */
export interface ActionMeta {
	readonly trace?: TraceMeta;
}

export interface Client {
	readonly [TypeId]: typeof TypeId;

	readonly makeActorAccessor: <
		Actions extends Action.AnyWithProps,
		EventDefs extends Event.Any = never,
		Connection extends Actor.AnyConnectionContract = never,
		Input extends Schema.Top = never,
	>(
		actor: Actor.Actor<string, Actions, EventDefs, Connection, Input>,
	) => Actor.Accessor<Actions, EventDefs, Connection, Input>;

	readonly auth: {
		/**
		 * Issues a token for the client's namespace with an explicit list of
		 * grants. The client's own token must be allowed to issue it.
		 */
		readonly issueToken: (
			options: RivetkitClient.IssueTokenOptions,
		) => Effect.Effect<RivetkitClient.IssuedToken, RivetError.RivetError>;
	};
}

export const Client: Context.Service<Client, Client> = Context.Service<Client>(
	"@rivetkit/effect/Client",
);

export const make = Effect.fnUntraced(function* (options: Options = {}) {
	const baseLogger = yield* getOrCreateBaseLogger;
	const rivetkitClient = yield* Effect.acquireRelease(
		Effect.sync(() => {
			// Raw RivetKit clients do not accept a logger in their config,
			// so install the Effect-selected logger before construction.
			RivetkitLog.configureBaseLogger(baseLogger);
			return RivetkitClient.createClient(options);
		}),
		(c) => Effect.promise(() => c.dispose()),
	);

	return Client.of({
		[TypeId]: TypeId,
		auth: {
			issueToken: (options) =>
				Effect.tryPromise({
					try: () => rivetkitClient.auth.issueToken(options),
					catch: RivetError.fromUnknown,
				}),
		},
		makeActorAccessor: <
			Actions extends Action.AnyWithProps,
			EventDefs extends Event.Any = never,
			Connection extends Actor.AnyConnectionContract = never,
			Input extends Schema.Top = never,
		>(
			actor: Actor.Actor<string, Actions, EventDefs, Connection, Input>,
		) => {
			const encodeOptions = makeEncodeAccessorOptions(actor);
			/**
			 * The RivetKit handle caches the actor it resolved, so every call
			 * of one `Handle` shares it.
			 */
			const makeLazyHandle = <Options extends AccessorOptions>(
				options: Options | undefined,
				makeRivetkitActorHandle: (
					options: Options | undefined,
				) => RivetkitActorHandle,
			) => {
				let rivetkitActorHandle: RivetkitActorHandle | undefined;
				return makeHandle(
					actor,
					Effect.suspend(() =>
						rivetkitActorHandle === undefined
							? encodeOptions(options).pipe(
									Effect.map((encodedOptions) => {
										rivetkitActorHandle =
											makeRivetkitActorHandle(
												encodedOptions,
											);
										return rivetkitActorHandle;
									}),
								)
							: Effect.succeed(rivetkitActorHandle),
					),
				);
			};

			return {
				getOrCreate: (
					key: Actor.AccessorKeyParam,
					options?: Actor.GetOrCreateOptions<
						Actor.AnyConnectionContract,
						Schema.Top
					>,
				) =>
					makeLazyHandle(options, (options) =>
						rivetkitClient.getOrCreate(actor.name, key, options),
					),
				get: (
					key: Actor.AccessorKeyParam,
					options?: Actor.GetOptions<Actor.AnyConnectionContract>,
				) =>
					makeLazyHandle(options, (options) =>
						rivetkitClient.get(actor.name, key, options),
					),
				getForId: (
					actorId: string,
					options?: Actor.GetOptions<Actor.AnyConnectionContract>,
				) =>
					makeLazyHandle(options, (options) =>
						rivetkitClient.getForId(actor.name, actorId, options),
					),
				create: (
					key: Actor.AccessorKeyParam,
					options?: Actor.CreateOptions<
						Actor.AnyConnectionContract,
						Schema.Top
					>,
				) =>
					encodeOptions(options).pipe(
						Effect.flatMap((encodedOptions) =>
							Effect.tryPromise({
								try: () =>
									rivetkitClient.create(
										actor.name,
										key,
										encodedOptions,
									),
								catch: RivetError.fromUnknown,
							}),
						),
						Effect.map((rivetkitActorHandle) =>
							makeHandle(
								actor,
								Effect.succeed(rivetkitActorHandle),
							),
						),
					),
				fork: (
					actorId: string,
					options?: Actor.ForkOptions<
						Actor.AnyConnectionContract,
						Schema.Top
					>,
				) =>
					encodeOptions(options).pipe(
						Effect.flatMap((encodedOptions) =>
							Effect.tryPromise({
								try: () =>
									rivetkitClient.fork(
										actor.name,
										actorId,
										encodedOptions,
									),
								catch: RivetError.fromUnknown,
							}),
						),
						Effect.map((rivetkitActorHandle) =>
							makeHandle(
								actor,
								Effect.succeed(rivetkitActorHandle),
							),
						),
					),
			} as unknown as Actor.Accessor<
				Actions,
				EventDefs,
				Connection,
				Input
			>;
		},
	});
});

type RivetkitActorHandle = Pick<
	RivetkitClient.ActorHandleRaw,
	| "action"
	| "connect"
	| "fetch"
	| "webSocket"
	| "resolve"
	| "getGatewayUrl"
	| "issueToken"
>;

type RivetkitFetchParameters = Parameters<RivetkitActorHandle["fetch"]>;

type AccessorOptions = {
	readonly params?: unknown;
	readonly input?: unknown;
	readonly createWithInput?: unknown;
};

const creationInputKeys = ["input", "createWithInput"] as const;

const makeInvalidEncodingError = (message: string) => (cause: unknown) =>
	new RivetError.RivetError({
		reason: new RivetError.InvalidEncoding({
			cause: new RivetkitErrors.RivetError(
				"encoding",
				"invalid",
				message,
				{
					public: true,
					metadata: cause,
				},
			),
		}),
	});

/**
 * Encodes `params` and the creation input with the codecs the actor decodes
 * them with.
 */
const makeEncodeAccessorOptions = (
	actor: Pick<
		Actor.Actor<string, any, any, any, any>,
		"connection" | "input"
	>,
) => {
	const encodeParams =
		actor.connection === undefined
			? undefined
			: Schema.encodeEffect(Schema.toCodecJson(actor.connection.params));
	const encodeInput =
		actor.input === undefined
			? undefined
			: Schema.encodeEffect(Schema.toCodecJson(actor.input));

	const encodeConnectionParams = <Options extends AccessorOptions>(
		options: Options,
	): Effect.Effect<Options, RivetError.RivetError> =>
		encodeParams === undefined
			? Effect.succeed(options)
			: (encodeParams(options.params).pipe(
					Effect.map((params) => ({ ...options, params })),
					Effect.mapError(
						makeInvalidEncodingError(
							"Could not encode connection params",
						),
					),
				) as Effect.Effect<Options, RivetError.RivetError>);

	const encodeCreationInput = <Options extends AccessorOptions>(
		options: Options,
	): Effect.Effect<Options, RivetError.RivetError> => {
		const key = creationInputKeys.find((key) => key in options);
		return encodeInput === undefined || key === undefined
			? Effect.succeed(options)
			: (encodeInput(options[key]).pipe(
					Effect.map((input) => ({ ...options, [key]: input })),
					Effect.mapError(
						makeInvalidEncodingError(
							"Could not encode creation input",
						),
					),
				) as Effect.Effect<Options, RivetError.RivetError>);
	};

	return <Options extends AccessorOptions>(
		options: Options | undefined,
	): Effect.Effect<Options | undefined, RivetError.RivetError> =>
		options === undefined
			? Effect.succeed(options)
			: encodeConnectionParams(options).pipe(
					Effect.flatMap(encodeCreationInput),
				);
};

type AnyClientActor = Pick<
	Actor.Actor<
		string,
		Action.AnyWithProps,
		Event.Any,
		Actor.AnyConnectionContract
	>,
	"name" | "actions" | "eventDefs" | "connection"
>;

const makeActionMethods = (
	actor: AnyClientActor,
	rivetkitActionTarget: Effect.Effect<
		Pick<RivetkitClient.ActorHandleRaw, "action">,
		RivetError.RivetError
	>,
) =>
	Record.fromIterableWith(actor.actions, (action) => {
		const encodePayload = Schema.encodeEffect(
			Schema.toCodecJson(action.payloadSchema),
		);
		const decodeSuccess = Schema.decodeUnknownEffect(
			Schema.toCodecJson(action.successSchema),
		);
		/** `authorize` refuses a stateless call with the connection error. */
		const classifyRivetkitActionFailure =
			makeRivetkitActionFailureClassifier(
				actor.connection === undefined
					? action.errorSchema
					: Schema.Union([
							action.errorSchema,
							actor.connection.error,
						]),
			);

		const rpcMethod = `${actor.name}/${action._tag}`;

		return [
			action._tag,
			Effect.fn(rpcMethod, {
				kind: "client",
				attributes: {
					"rpc.system.name": rpcSystem,
					"rpc.method": rpcMethod,
				},
			})(function* (payload: unknown) {
				const span = yield* Effect.currentSpan;
				const meta: ActionMeta = {
					trace: {
						traceId: span.traceId,
						spanId: span.spanId,
						sampled: span.sampled,
					},
				};
				const encodedPayload = yield* encodePayload(payload).pipe(
					Effect.mapError(
						makeInvalidEncodingError(
							"Could not encode action payload",
						),
					),
				);
				const resolvedRivetkitActionTarget =
					yield* rivetkitActionTarget;

				const encodedSuccess = yield* Effect.tryPromise((abortSignal) =>
					resolvedRivetkitActionTarget.action({
						name: action._tag,
						args: [encodedPayload, meta],
						signal: abortSignal,
					}),
				).pipe(
					Effect.catch((unknownError) =>
						classifyRivetkitActionFailure(unknownError.cause).pipe(
							Effect.flatMap(Effect.fail),
						),
					),
				);

				return yield* decodeSuccess(encodedSuccess).pipe(Effect.orDie);
			}),
		];
	});

const makeHandle = <
	Actions extends Action.AnyWithProps,
	EventDefs extends Event.Any,
	Connection extends Actor.AnyConnectionContract,
>(
	actor: Actor.Actor<string, Actions, EventDefs, Connection, any>,
	rivetkitActorHandle: Effect.Effect<
		RivetkitActorHandle,
		RivetError.RivetError
	>,
): Actor.Handle<Actions, EventDefs, Connection> =>
	({
		...makeActionMethods(actor, rivetkitActorHandle),
		connect: makeConnect(actor, rivetkitActorHandle),
		fetch: makeFetch(rivetkitActorHandle),
		webSocket: makeWebSocket(rivetkitActorHandle),
		resolve: callRivetkitActorHandle(rivetkitActorHandle, (handle) =>
			handle.resolve(),
		),
		getGatewayUrl: callRivetkitActorHandle(rivetkitActorHandle, (handle) =>
			handle.getGatewayUrl(),
		),
		issueToken: (options?: RivetkitClient.ActorIssueTokenOptions) =>
			callRivetkitActorHandle(rivetkitActorHandle, (handle) =>
				handle.issueToken(options),
			),
	}) as unknown as Actor.Handle<Actions, EventDefs, Connection>;

const callRivetkitActorHandle = <A>(
	rivetkitActorHandle: Effect.Effect<
		RivetkitActorHandle,
		RivetError.RivetError
	>,
	call: (handle: RivetkitActorHandle) => Promise<A>,
) =>
	rivetkitActorHandle.pipe(
		Effect.flatMap((resolvedRivetkitActorHandle) =>
			Effect.tryPromise({
				try: () => call(resolvedRivetkitActorHandle),
				catch: RivetError.fromUnknown,
			}),
		),
	);

const makeFetch =
	(
		rivetkitActorHandle: Effect.Effect<
			Pick<RivetkitActorHandle, "fetch">,
			RivetError.RivetError
		>,
	) =>
	(input: RivetkitFetchParameters[0], init?: RivetkitFetchParameters[1]) =>
		rivetkitActorHandle.pipe(
			Effect.flatMap((resolvedRivetkitActorHandle) =>
				Effect.tryPromise({
					try: (signal) =>
						resolvedRivetkitActorHandle.fetch(input, {
							...init,
							signal,
						}),
					catch: RivetError.fromUnknown,
				}),
			),
		);

const makeWebSocket =
	(
		rivetkitActorHandle: Effect.Effect<
			Pick<RivetkitActorHandle, "webSocket">,
			RivetError.RivetError
		>,
	) =>
	(path?: string, protocols?: string | Array<string>) =>
		Socket.fromWebSocket(
			Effect.acquireRelease(
				rivetkitActorHandle.pipe(
					Effect.flatMap((resolvedRivetkitActorHandle) =>
						Effect.tryPromise({
							try: () =>
								resolvedRivetkitActorHandle.webSocket(
									path,
									protocols,
								),
							catch: RivetError.fromUnknown,
						}),
					),
					Effect.mapError(
						(cause) =>
							new Socket.SocketError({
								reason: new Socket.SocketOpenError({
									kind: "Unknown",
									cause,
								}),
							}),
					),
				),
				(rivetkitWebSocket) =>
					Effect.sync(() => rivetkitWebSocket.close(1000)),
			),
		);

const makeConnect = (
	actor: AnyClientActor,
	rivetkitActorHandle: Effect.Effect<
		Pick<RivetkitActorHandle, "connect">,
		RivetError.RivetError
	>,
) => {
	const classifyConnectFailure = makeRivetkitActionFailureClassifier(
		actor.connection?.error ?? Schema.Never,
	);

	return Effect.gen(function* () {
		let closedByScope = false;
		let lastError: unknown;
		const resolvedRivetkitActorHandle = yield* rivetkitActorHandle;
		const conn = yield* Effect.acquireRelease(
			Effect.sync(
				() =>
					resolvedRivetkitActorHandle.connect() as unknown as RivetkitClient.ActorConnRaw,
			),
			(conn) =>
				Effect.promise(() => {
					closedByScope = true;
					return conn.dispose();
				}),
		);
		conn.onError((error) => {
			lastError = error;
		});

		/** RivetKit goes `idle` once it stops retrying a failed open. */
		yield* Effect.callback<void, unknown>((resume) => {
			const unsubscribe = () => {
				offOpen();
				offStatusChange();
			};
			const offOpen = conn.onOpen(() => {
				unsubscribe();
				resume(Effect.void);
			});
			const offStatusChange = conn.onStatusChange((status) => {
				if (status !== "idle") return;
				unsubscribe();
				resume(Effect.fail(lastError));
			});
			return Effect.sync(unsubscribe);
		}).pipe(
			Effect.catch((cause) =>
				classifyConnectFailure(cause).pipe(Effect.flatMap(Effect.fail)),
			),
		);

		/**
		 * Subscribes on the stream's first pull without forking, so a stream
		 * started before an action call sees the events that call sends.
		 */
		const makeEventStream = (event: Event.Any) => {
			const decodePayload = Schema.decodeUnknownEffect(
				Schema.toCodecJson(event.payloadSchema),
			);
			return Stream.unwrap(
				Effect.gen(function* () {
					const queue = yield* Queue.make<
						unknown,
						RivetError.RivetError | Cause.Done
					>();
					const close = () =>
						closedByScope
							? Queue.endUnsafe(queue)
							: Queue.failCauseUnsafe(
									queue,
									Cause.fail(
										RivetError.fromUnknown(lastError),
									),
								);
					yield* Effect.acquireRelease(
						Effect.sync(() => {
							const offEvent = conn.on(
								event._tag,
								(payload: unknown) => {
									Queue.offerUnsafe(queue, payload);
								},
							);
							const offStatusChange = conn.onStatusChange(
								(status) => {
									if (status === "idle") close();
								},
							);
							/**
							 * When `canSubscribe` refuses a subscription,
							 * RivetKit closes the connection with
							 * `auth.forbidden` and keeps reconnecting.
							 */
							const offError = conn.onError((error) => {
								const rivetError =
									RivetError.fromUnknown(error);
								if (
									rivetError.group === "auth" &&
									rivetError.code === "forbidden"
								) {
									Queue.failCauseUnsafe(
										queue,
										Cause.fail(rivetError),
									);
								}
							});
							if (conn.connStatus === "idle") close();
							return () => {
								offEvent();
								offStatusChange();
								offError();
							};
						}),
						(unsubscribe) => Effect.sync(unsubscribe),
					);
					return Stream.fromQueue(queue);
				}),
			).pipe(
				Stream.mapEffect((payload) =>
					event.hasPayload
						? decodePayload(payload).pipe(Effect.orDie)
						: Effect.void,
				),
			);
		};

		return {
			...makeActionMethods(actor, Effect.succeed(conn)),
			events: Record.fromIterableWith(actor.eventDefs, (event) => [
				event._tag,
				makeEventStream(event),
			]),
		};
	});
};

export const layer = (options: Options = {}): Layer.Layer<Client> =>
	Layer.unwrap(
		Effect.map(getOrCreateBaseLogger, (baseLogger) =>
			Layer.effect(Client, make(options)).pipe(
				Layer.provideMerge(RivetLogger.layerFromPino(baseLogger)),
			),
		),
	);

const decodeActionErrorEnvelope = Schema.decodeUnknownEffect(
	ActionErrorEnvelope.ActionErrorEnvelope,
);

/** @internal */
export const makeRivetkitActionFailureClassifier = <
	ActionErrorSchema extends Schema.Codec<unknown, unknown, unknown, unknown>,
>(
	actionErrorSchema: ActionErrorSchema,
): ((
	cause: unknown,
) => Effect.Effect<
	ActionErrorSchema["Type"] | RivetError.RivetError,
	never,
	ActionErrorSchema["DecodingServices"]
>) => {
	const decodeActionError = Schema.decodeUnknownEffect(
		Schema.toCodecJson(actionErrorSchema),
	);

	return Effect.fnUntraced(function* (
		cause: unknown,
	): Effect.fn.Return<
		ActionErrorSchema["Type"] | RivetError.RivetError,
		never,
		ActionErrorSchema["DecodingServices"]
	> {
		// In the case where the `cause` is not a `RivetError`. In principle, this shouldn't happen.
		if (!RivetkitErrors.isRivetErrorLike(cause)) {
			return RivetError.fromUnknown(cause);
		}

		const rivetkitRivetError = RivetkitErrors.toRivetError(cause);

		const actionErrorEnvelope = yield* Effect.result(
			decodeActionErrorEnvelope(rivetkitRivetError.metadata),
		);

		// If the error's `metadata` is not a valid action error envelope, then
		// it means it's not a user-declared action error.
		if (Result.isFailure(actionErrorEnvelope)) {
			return RivetError.fromRivetkitRivetError(rivetkitRivetError);
		}

		const actionErrorResult = yield* Effect.result(
			decodeActionError(actionErrorEnvelope.success.error),
		);

		// The envelope was valid, but the inner payload doesn't match the
		// declared schema — surface as `ActionErrorDecodeFailed`
		if (Result.isFailure(actionErrorResult)) {
			return new RivetError.RivetError({
				reason: new RivetError.ActionErrorDecodeFailed({
					cause: actionErrorResult.failure,
					rivetError: rivetkitRivetError,
				}),
			});
		}

		// Successfully decoded user-declared action error
		return actionErrorResult.success;
	});
};
