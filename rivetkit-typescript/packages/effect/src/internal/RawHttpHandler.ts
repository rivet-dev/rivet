import { Cause, type Context, Effect, Exit, type Fiber } from "effect";
import { HttpEffect, HttpServerRequest, HttpServerResponse } from "effect/http";
import type * as Rivetkit from "rivetkit";
import { makeActorAbortedError } from "./ActionDispatcher.ts";
import { makeActorLogAnnotations } from "./logging.ts";

/**
 * Handles the raw HTTP requests sent to an actor. A failure or a defect
 * becomes a `500` response, or the error's own response if it is an
 * `effect/http` `Respondable`.
 */
export type HttpHandler<R> = Effect.Effect<
	HttpServerResponse.HttpServerResponse,
	unknown,
	R
>;

const respondByRequest = new WeakMap<
	HttpServerRequest.HttpServerRequest,
	(response: Response) => void
>();

/**
 * The effect that answers one request with `handler`. `toHandled` runs its
 * effect uninterruptibly.
 */
export const toRequestEffect = (handler: HttpHandler<any>) =>
	HttpEffect.toHandled(Effect.interruptible(handler), (request, response) =>
		Effect.contextWith((context: Context.Context<never>) =>
			Effect.sync(() => {
				const withoutBody = request.method === "HEAD";
				respondByRequest.get(request)?.(
					HttpServerResponse.toWeb(
						HttpServerResponse.omitsBody(response, withoutBody)
							? response
							: HttpEffect.scopeTransferToStream(response),
						{ withoutBody, context },
					),
				);
			}),
		),
	);

export type RequestEffect = ReturnType<typeof toRequestEffect>;

type Instance = {
	readonly runFork: <A, E>(
		effect: Effect.Effect<A, E, any>,
		options?: Effect.RunOptions,
	) => Fiber.Fiber<A, E>;
	readonly http: RequestEffect | undefined;
};

type RivetkitContext = {
	readonly actorId: string;
	readonly name: string;
	readonly key: Rivetkit.ActorKey;
};

/**
 * Builds the RivetKit `onRequest` callback that runs the instance's `http`
 * hook. Without one it answers `404`, as RivetKit does without `onRequest`.
 */
export const makeRivetkitOnRequest = ({
	getInstance,
}: {
	readonly getInstance: (actorId: string) => Instance | undefined;
}): ((
	c: RivetkitContext,
	request: Request,
) => Response | Promise<Response>) => {
	return (c, request) => {
		const instance = getInstance(c.actorId);
		if (!instance) throw makeActorAbortedError();

		const { http } = instance;
		if (!http) return new Response(null, { status: 404 });

		return new Promise<Response>((resolve, reject) => {
			const httpServerRequest = HttpServerRequest.fromWeb(request);
			respondByRequest.set(httpServerRequest, resolve);
			instance
				.runFork(
					http.pipe(
						Effect.provideService(
							HttpServerRequest.HttpServerRequest,
							httpServerRequest,
						),
						Effect.annotateLogs(makeActorLogAnnotations(c)),
					),
					{ signal: request.signal },
				)
				.addObserver((exit) => {
					if (Exit.isFailure(exit)) reject(Cause.squash(exit.cause));
				});
		});
	};
};
