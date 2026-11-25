import {
	Array as Arr,
	type Context,
	Effect,
	Exit,
	Option,
	Record,
	Schema,
} from "effect";
import * as Rivetkit from "rivetkit";
import type {
	ActionConnection,
	Actor,
	ActorConnections,
	AnyConnectionContract,
} from "../Actor.ts";
import * as RivetError from "../RivetError.ts";
import {
	makeActorAbortedError,
	runInstanceEffect,
} from "./ActionDispatcher.ts";
import * as ActionErrorEnvelope from "./ActionErrorEnvelope.ts";
import type * as ActorInstanceManager from "./ActorInstanceManager.ts";
import { makeActorLogAnnotations } from "./logging.ts";

/** The connection hooks as `MyActor.of` gets them. */
export type UnboundConnectionHooks = {
	readonly authorize?: (
		params: any,
		request: Option.Option<Request>,
	) => Effect.Effect<void, any, any>;
	readonly onConnect?: (connection: any) => Effect.Effect<void, never, any>;
	readonly onDisconnect?: (
		connection: any,
	) => Effect.Effect<void, never, any>;
	readonly canSubscribe?: {
		readonly [eventTag: string]:
			| ((connection: any) => Effect.Effect<boolean, never, any>)
			| undefined;
	};
};

/** The connection hooks of one wake, bound to the context of its wake effect. */
export type ConnectionHooks = {
	/** `MyActor.of` requires it for an actor that declares `connection`, the only kind that runs it. */
	readonly authorize: (
		params: unknown,
		request: Option.Option<Request>,
	) => Effect.Effect<void, unknown>;
	readonly onConnect:
		| ((connection: ActionConnection<unknown>) => Effect.Effect<void>)
		| undefined;
	readonly onDisconnect:
		| ((connection: ActionConnection<unknown>) => Effect.Effect<void>)
		| undefined;
	readonly canSubscribe: {
		readonly [eventTag: string]:
			| ((
					connection: ActionConnection<unknown>,
			  ) => Effect.Effect<boolean>)
			| undefined;
	};
};

/**
 * Each hook keeps the context of the wake effect, so it runs with the wake's
 * services, scope and log annotations.
 */
export const bindToWakeContext = (
	actorName: string,
	hooks: UnboundConnectionHooks,
): Effect.Effect<ConnectionHooks> =>
	Effect.contextWith((context: Context.Context<never>) => {
		const bind =
			<Args extends ReadonlyArray<any>, A>(
				name: keyof ConnectionHooks,
				hook: (...args: Args) => Effect.Effect<A, unknown, any>,
			) =>
			(...args: Args) =>
				hook(...args).pipe(
					Effect.withSpan(`${actorName}/${name}`, { kind: "server" }),
					Effect.provideContext(context),
				);

		return Effect.succeed({
			authorize: bind(
				"authorize",
				hooks.authorize as ConnectionHooks["authorize"],
			),
			onConnect: hooks.onConnect && bind("onConnect", hooks.onConnect),
			onDisconnect:
				hooks.onDisconnect && bind("onDisconnect", hooks.onDisconnect),
			canSubscribe: Record.map(
				hooks.canSubscribe ?? {},
				(hook) => hook && bind("canSubscribe", hook),
			),
		} as ConnectionHooks);
	});

export const makeActorConnections = (
	conns: ReadonlyMap<string, Rivetkit.AnyConn>,
	resolveConnection: ActorInstanceManager.Instance<
		unknown,
		any
	>["connection"],
): ActorConnections<AnyConnectionContract> => {
	const list = Effect.suspend(() =>
		Effect.forEach(Array.from(conns.values()), (conn) =>
			resolveConnection(conn).pipe(
				Effect.asSome,
				Effect.catch((error) =>
					Effect.logWarning(
						"Skipped a connection whose params do not decode",
						error,
					).pipe(
						Effect.annotateLogs({ connectionId: conn.id }),
						Effect.as(Option.none()),
					),
				),
			),
		),
	).pipe(Effect.map(Arr.getSomes));

	const disconnect = (connection: ActionConnection, reason?: string) =>
		Effect.tryPromise({
			try: async () => {
				await conns.get(connection.id)?.disconnect(reason);
			},
			catch: RivetError.fromUnknown,
		});

	return { list, disconnect };
};

type Instance = Pick<
	ActorInstanceManager.Instance<unknown, any>,
	| "runFork"
	| "connectionHooks"
	| "openConnection"
	| "closeConnection"
	| "isConnected"
	| "disconnectConnection"
	| "connection"
>;

type RivetkitContext = {
	readonly actorId: string;
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
	readonly request?: Request;
};

export const makeDecodeConnectionParams = (
	actor: Pick<Actor<string, any, any, any>, "name" | "connection">,
): ActorInstanceManager.DecodeConnectionParams | undefined => {
	if (actor.connection === undefined) return undefined;

	const decodeParams = Schema.decodeUnknownEffect(
		Schema.toCodecJson(actor.connection.params),
	);
	return (params) =>
		decodeParams(params).pipe(
			Effect.tapError((schemaError) =>
				Effect.logDebug(
					`Invalid connection params for actor ${actor.name}`,
					schemaError,
				),
			),
			Effect.mapError(
				() =>
					new Rivetkit.RivetError(
						"request",
						"invalid",
						`Invalid connection params for actor ${actor.name}`,
					),
			),
		) as Effect.Effect<unknown, Rivetkit.RivetError>;
};

/**
 * Builds the RivetKit connection callbacks. RivetKit runs them only after
 * `onWake` returned, so the instance has the wake's hooks by then.
 */
export const makeRivetkitCallbacks = ({
	actor,
	getInstance,
}: {
	readonly actor: Pick<
		Actor<string, any, any, any>,
		"name" | "connection" | "eventDefs"
	>;
	readonly getInstance: (actorId: string) => Instance | undefined;
}) => {
	const decodeParams = makeDecodeConnectionParams(actor);
	const encodeError =
		actor.connection === undefined
			? undefined
			: Schema.encodeEffect(Schema.toCodecJson(actor.connection.error));

	const requireInstance = (c: RivetkitContext) => {
		const instance = getInstance(c.actorId);
		if (!instance) throw makeActorAbortedError();
		return instance;
	};

	const onBeforeConnect =
		decodeParams === undefined || encodeError === undefined
			? undefined
			: async (c: RivetkitContext, params: unknown) => {
					const instance = requireInstance(c);
					await runInstanceEffect(
						instance.runFork,
						decodeParams(params).pipe(
							Effect.flatMap((decodedParams) =>
								instance.connectionHooks
									.authorize(
										decodedParams,
										Option.fromNullishOr(c.request),
									)
									.pipe(
										Effect.catch((error) =>
											encodeError(error).pipe(
												Effect.orDie,
												Effect.flatMap((encodedError) =>
													Effect.fail(
														ActionErrorEnvelope.toUserError(
															encodedError,
															"Connection refused",
														),
													),
												),
											),
										),
									),
							),
							Effect.annotateLogs(makeActorLogAnnotations(c)),
						),
					);
				};

	const onConnect = async (
		c: RivetkitContext,
		conn: ActorInstanceManager.RivetkitConn,
	) => {
		const instance = requireInstance(c);
		const { onConnect } = instance.connectionHooks;
		await runInstanceEffect(
			instance.runFork,
			instance.openConnection(conn).pipe(
				Effect.flatMap((connection) =>
					onConnect === undefined
						? Effect.void
						: onConnect(connection),
				),
				/**
				 * RivetKit reports no disconnect for a connection whose
				 * `onConnect` failed or ran past its timeout.
				 */
				Effect.onExit((exit) =>
					Exit.isFailure(exit)
						? instance.closeConnection(conn.id)
						: instance.isConnected(conn.id)
							? Effect.void
							: Effect.ignore(
									instance.disconnectConnection(conn),
								),
				),
			),
		);
	};

	const onDisconnect = async (
		c: RivetkitContext,
		conn: ActorInstanceManager.RivetkitConn,
	) => {
		const instance = getInstance(c.actorId);
		// Teardown already ran the hook for this connection.
		if (!instance) return;

		await runInstanceEffect(
			instance.runFork,
			instance.disconnectConnection(conn),
		);
	};

	/**
	 * RivetKit runs `canSubscribe` only for the events in `events`, so each
	 * declared event gets a callback that runs the hook of the wake, if any.
	 */
	const canSubscribe =
		(eventTag: string) =>
		async (
			c: RivetkitContext & {
				readonly conn: ActorInstanceManager.RivetkitConn;
			},
		): Promise<boolean> => {
			const instance = requireInstance(c);
			const hooks = instance.connectionHooks.canSubscribe;
			const hook = Object.hasOwn(hooks, eventTag)
				? hooks[eventTag]
				: undefined;
			if (hook === undefined) return true;

			return runInstanceEffect(
				instance.runFork,
				instance
					.connection(c.conn)
					.pipe(
						Effect.flatMap(hook),
						Effect.annotateLogs(makeActorLogAnnotations(c)),
					),
			);
		};
	const events =
		actor.eventDefs.length === 0
			? undefined
			: Object.fromEntries(
					actor.eventDefs.map((event) => [
						event._tag,
						{ canSubscribe: canSubscribe(event._tag) },
					]),
				);

	return { onBeforeConnect, onConnect, onDisconnect, events };
};
