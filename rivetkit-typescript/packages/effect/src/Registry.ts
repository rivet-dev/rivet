import { Context, Effect, Layer, type Scope } from "effect";
import {
	HttpEffect,
	type HttpMiddleware,
	type HttpServerError,
	type HttpServerRequest,
	type HttpServerResponse,
} from "effect/unstable/http";
import * as Rivetkit from "rivetkit";
import { configureBaseLogger, type Logger as PinoLogger } from "rivetkit/log";
import * as Client from "./Client.ts";
import { BaseLogger, getOrCreateBaseLogger } from "./internal/logging.ts";
import * as RivetLogger from "./RivetLogger.ts";

const TypeId = "~@rivetkit/effect/Registry";
type ServerlessOptions = NonNullable<
	Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>["serverless"]
>;

export type Options = Pick<
	Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>,
	"endpoint" | "token" | "namespace" | "noWelcome"
>;

export interface Registry {
	readonly [TypeId]: typeof TypeId;

	readonly options: Options;

	readonly baseLogger: PinoLogger;

	readonly rivetkitActors: Map<string, Rivetkit.AnyActorDefinition>;
}

export const Registry: Context.Service<Registry, Registry> =
	Context.Service<Registry>("@rivetkit/effect/Registry");

const make = (options: Options, baseLogger: PinoLogger): Registry => {
	return Registry.of({
		[TypeId]: TypeId,
		options,
		baseLogger,
		rivetkitActors: new Map(),
	});
};

export const layer = (options: Options = {}): Layer.Layer<Registry> =>
	Layer.effect(
		Registry,
		Effect.map(getOrCreateBaseLogger, (baseLogger) =>
			make(options, baseLogger),
		),
	);

const setupRivetkitRegistry = (
	registry: Registry,
	options?: {
		readonly serverless?: ServerlessOptions | undefined;
	},
) => {
	configureBaseLogger(registry.baseLogger);
	return Rivetkit.setup({
		use: Object.fromEntries(registry.rivetkitActors),
		...registry.options,
		logging: { baseLogger: registry.baseLogger },
		...(options?.serverless === undefined
			? {}
			: { serverless: options.serverless }),
	});
};

/**
 * Runs an actor registration layer against the configured engine.
 *
 * The actor layer is built in the server layer scope. Registered Rivet Actors
 * are collected from `Registry`, materialized into a single underlying RivetKit
 * registry, and started.
 */
export const serve = <E, R>(
	actorsLayer: Layer.Layer<never, E, R>,
): Layer.Layer<never, E, R | Registry> =>
	Layer.effectDiscard(
		Effect.gen(function* () {
			const registry = yield* Registry;
			const baseLogger = registry.baseLogger;
			yield* Layer.build(
				actorsLayer.pipe(
					Layer.provideMerge(RivetLogger.layerFromPino(baseLogger)),
				),
			);
			const rivetkitRegistry = setupRivetkitRegistry(registry);
			yield* Effect.sync(() => rivetkitRegistry.start());
		}),
	);

/**
 * In-process test runtime. Boots the rivetkit registry against the
 * configured engine, waits for `/health` to answer, and provides
 * `Client` from the same Layer so consumers don't need to wire
 * `Client.layer` separately. Mirrors `Registry.start` plus test-mode
 * flags and a scoped client dispose. The registry itself is leaked
 * to process exit because the public rivetkit `Registry` doesn't
 * expose a public `shutdown()` today; only the SIGINT handler can
 * drive `#runShutdown`. This matches `setupTest`'s existing behavior.
 */
export const test: Layer.Layer<Client.Client, never, Registry> = Layer.effect(
	Client.Client,
	Effect.gen(function* () {
		const registry = yield* Registry;
		const rivetkitRegistry = setupRivetkitRegistry(registry);
		rivetkitRegistry.config.test = {
			...rivetkitRegistry.config.test,
			enabled: true,
		};
		rivetkitRegistry.config.noWelcome = true;
		// Auto-spawn the engine when no endpoint was provided, so
		// `Registry.test` works out of the box without requiring the
		// caller to start an engine externally. If the user wired an
		// explicit endpoint via `Registry.layer({ endpoint: ... })`,
		// honor it and skip the local spawn.
		if (registry.options.endpoint === undefined) {
			rivetkitRegistry.config.startEngine = true;
		}
		yield* Effect.sync(() => rivetkitRegistry.start());

		// The rivetkitRegistry itself is leaked until process exit (matches
		// setupTest's behavior). The public Rivetkit.Registry doesn't
		// expose a shutdown method; only the SIGINT handler can drive the
		// inner .shutdown(). Disposing the client is the only cleanup we
		// can do cleanly today.
		//
		// When the engine was auto-spawned, propagate its resolved
		// endpoint to the client so `createClient` doesn't fall back
		// to its (warning-emitting) default.
		const resolvedEndpoint = rivetkitRegistry.parseConfig().endpoint;

		return yield* Client.make({
			...registry.options,
			endpoint: registry.options.endpoint ?? resolvedEndpoint,
		}).pipe(Effect.provideService(BaseLogger, registry.baseLogger));
	}),
);

const makeHttpEffect = (
	registry: Registry,
	options?: ToHttpEffectOptions,
): Effect.Effect<
	HttpServerResponse.HttpServerResponse,
	HttpServerError.HttpServerError,
	HttpServerRequest.HttpServerRequest
> => {
	const rivetkitRegistry = setupRivetkitRegistry(registry, {
		serverless: options,
	});
	return HttpEffect.fromWebHandler((request) =>
		rivetkitRegistry.handler(request),
	);
};

export type ToHttpEffectOptions = ServerlessOptions;

/**
 * Builds a scoped Effect HTTP handler from a registry layer.
 *
 * The registry layer is built once in the surrounding scope. Registered Rivet
 * Actors are materialized into a single underlying RivetKit registry, and each
 * request is delegated to that registry's serverless handler.
 */
export const toHttpEffect = Effect.fnUntraced(function* <E>(
	registryLayer: Layer.Layer<Registry, E>,
	options?: ToHttpEffectOptions,
): Effect.fn.Return<
	Effect.Effect<
		HttpServerResponse.HttpServerResponse,
		HttpServerError.HttpServerError,
		HttpServerRequest.HttpServerRequest
	>,
	E,
	Scope.Scope
> {
	const context = yield* Layer.build(
		registryLayer.pipe(Layer.provideMerge(RivetLogger.defaultLayer)),
	);
	// @effect-diagnostics-next-line returnEffectInGen:off
	return makeHttpEffect(Context.get(context, Registry), options).pipe(
		Effect.provide(context),
	);
});

export type ToWebHandlerOptions = ServerlessOptions & {
	/**
	 * Effect HTTP middleware applied around the generated handler.
	 */
	readonly middleware?: HttpMiddleware.HttpMiddleware | undefined;
	/**
	 * Memo map used while building the registry layer.
	 */
	readonly memoMap?: Layer.MemoMap | undefined;
};

/**
 * Builds a Fetch-compatible request handler from a registry layer.
 *
 * This is the serverless entrypoint for the Effect SDK. The registry layer must
 * provide `Registry`, usually by composing actor layers with `Registry.layer`
 * via `Layer.provideMerge`.
 */
export const toWebHandler = <E>(
	registryLayer: Layer.Layer<Registry, E>,
	options?: ToWebHandlerOptions,
) => {
	const { middleware, memoMap } = options ?? {};
	let serverlessOptions: ServerlessOptions | undefined;
	if (options !== undefined) {
		const {
			middleware: _middleware,
			memoMap: _memoMap,
			...handlerOptions
		} = options;
		serverlessOptions = handlerOptions;
	}

	const registryLayerWithLogging = registryLayer.pipe(
		Layer.provideMerge(RivetLogger.defaultLayer),
	);

	return HttpEffect.toWebHandlerLayerWith(registryLayerWithLogging, {
		toHandler: (context) =>
			Effect.succeed(
				makeHttpEffect(
					Context.get(context, Registry),
					serverlessOptions,
				).pipe(Effect.provide(context)),
			),
		middleware,
		memoMap,
	});
};
