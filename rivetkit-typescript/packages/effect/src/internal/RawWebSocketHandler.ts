import { Cause, Effect, Exit, type Fiber, identity, Option } from "effect";
import { HttpServerRequest } from "effect/http";
import { Socket } from "effect/socket";
import type * as Rivetkit from "rivetkit";
import { makeActorAbortedError } from "./ActionDispatcher.ts";
import { makeActorLogAnnotations } from "./logging.ts";

/**
 * Handles one raw WebSocket connection to an actor. The connection closes
 * when the handler exits.
 */
export type WebSocketHandler<R> = Effect.Effect<void, unknown, R>;

type Instance = {
	readonly runFork: <A, E>(
		effect: Effect.Effect<A, E, any>,
		options?: Effect.RunOptions,
	) => Fiber.Fiber<A, E>;
	readonly webSocket: WebSocketHandler<any> | undefined;
};

type RivetkitContext = {
	readonly actorId: string;
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
	readonly request?: Request;
};

const isSocketClosed = (error: unknown) =>
	Socket.isSocketError(error) && error.reason._tag === "SocketCloseError";

/**
 * Builds the RivetKit `onWebSocket` callback that runs the instance's
 * `webSocket` hook. Without one it leaves the socket alone, as RivetKit does
 * without `onWebSocket`.
 */
export const makeRivetkitOnWebSocket = ({
	actorName,
	getInstance,
}: {
	readonly actorName: string;
	readonly getInstance: (actorId: string) => Instance | undefined;
}): ((c: RivetkitContext, webSocket: Socket.WebSocketLike) => void) => {
	return (c, webSocket) => {
		const instance = getInstance(c.actorId);
		if (!instance) throw makeActorAbortedError();

		const handler = instance.webSocket;
		if (!handler) return;

		/**
		 * RivetKit delivers messages from the moment the connection opens, so
		 * the reader is acquired before the handler runs. Later acquisitions
		 * share it.
		 */
		const handleSocket = Effect.gen(function* () {
			const socket = yield* Socket.fromWebSocket(
				Effect.succeed(webSocket),
			);
			const reader = yield* socket.reader;
			return yield* handler.pipe(
				Effect.provideService(
					Socket.Socket,
					Socket.make({
						reader: Effect.succeed(reader),
						writer: socket.writer,
					}),
				),
			);
		});

		/**
		 * An interrupted handler leaves the socket to RivetKit: the client
		 * closed it, or the actor is going to sleep and the socket may
		 * hibernate.
		 */
		const closeSocket = (exit: Exit.Exit<void, unknown>) => {
			if (Exit.isSuccess(exit)) {
				return Effect.sync(() => webSocket.close(1000));
			}
			if (
				Cause.hasInterruptsOnly(exit.cause) ||
				Option.exists(Exit.findErrorOption(exit), isSocketClosed)
			) {
				return Effect.void;
			}
			return Effect.logError(
				"Raw WebSocket handler failed",
				exit.cause,
			).pipe(
				Effect.andThen(
					Effect.sync(() => webSocket.close(1011, "Internal error")),
				),
			);
		};

		const fiber = instance.runFork(
			handleSocket.pipe(
				c.request
					? Effect.provideService(
							HttpServerRequest.HttpServerRequest,
							HttpServerRequest.fromWeb(c.request),
						)
					: identity,
				Effect.scoped,
				Effect.onExit(closeSocket),
				Effect.withSpan(`${actorName}/webSocket`, { kind: "server" }),
				Effect.annotateLogs(makeActorLogAnnotations(c)),
			),
		);
		webSocket.addEventListener("close", () => fiber.interruptUnsafe());
	};
};
