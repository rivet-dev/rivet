import { Context, Effect, Layer, type Scope } from "effect";
import {
	HttpEffect,
	type HttpMiddleware,
	type HttpServerError,
	type HttpServerRequest,
	HttpServerResponse,
} from "effect/http";
import * as Rivetkit from "rivetkit";
import type * as RivetkitLog from "rivetkit/log";
import * as Client from "./Client.ts";
import { BaseLogger, getOrCreateBaseLogger } from "./internal/logging.ts";
import { RegistryService } from "./internal/RegistryService.ts";
import { makeRivetkitActor } from "./internal/RivetkitActorDefinition.ts";
import * as RivetLogger from "./RivetLogger.ts";

const TypeId = "~@rivetkit/effect/Registry";
type ServerlessOptions = NonNullable<
	Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>["serverless"]
>;

type RivetkitRegistryConfigInput =
	Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>;

/**
 * `startEngine`, `startServices` and `runtime` are the RivetKit options for
 * a node that runs on its own, for example in a Docker image: set
 * `startEngine: false`, `startServices: false` and `runtime: "native"`.
 * `node.version` is the version of this node. Increase it on every deploy.
 */
export type Options = Pick<
	RivetkitRegistryConfigInput,
	| "endpoint"
	| "token"
	| "namespace"
	| "noWelcome"
	| "sqlite"
	| "startEngine"
	| "startServices"
	| "runtime"
> & {
	readonly node?: Pick<
		NonNullable<RivetkitRegistryConfigInput["node"]>,
		"version"
	>;
};

export interface Registry {
	readonly [TypeId]: typeof TypeId;

	readonly options: Options;

	readonly baseLogger: RivetkitLog.Logger;

	readonly rivetkitActors: Map<string, Rivetkit.AnyActorDefinition>;

	/** @internal The RivetKit registry that `serve` or `test` made. */
	readonly rivetkitRegistry: {
		current: Rivetkit.Registry<Rivetkit.RegistryActors> | undefined;
	};

	/** @internal */
	readonly makeRivetkitActor: typeof makeRivetkitActor;
}

export const Registry: Context.Service<Registry, Registry> = RegistryService;

/**
 * Creates the shared registry that actor layers register into.
 *
 * @remarks
 * Actor layers created with {@link Actor.Actor.toLayer|`Actor.toLayer()`}
 * register into this service. {@link serve|`Registry.serve()`} and
 * {@link test|`Registry.test`} later use the collected actors to create the
 * underlying RivetKit runtime.
 *
 * @param options - RivetKit runtime options shared by the registered actors.
 *
 * @returns A layer that provides the {@link Registry|`Registry.Registry`}
 * service.
 *
 * @example
 * ```ts
 * import { Registry } from "@rivetkit/effect"
 * import { Layer } from "effect"
 *
 * const ActorsLayer = Layer.mergeAll(ChatRoomLive, ModeratorLive)
 *
 * const MainLayer = Registry.serve(ActorsLayer).pipe(
 *   Layer.provide(Registry.layer({ endpoint: "http://127.0.0.1:6420" }))
 * )
 * ```
 *
 * @see {@link serve|`Registry.serve()`} for starting a long-running actor
 * server.
 */
export const layer = (options: Options = {}): Layer.Layer<Registry> =>
	Layer.effect(
		Registry,
		Effect.map(getOrCreateBaseLogger, (baseLogger) =>
			Registry.of({
				[TypeId]: TypeId,
				options,
				baseLogger,
				rivetkitActors: new Map(),
				rivetkitRegistry: { current: undefined },
				makeRivetkitActor,
			}),
		),
	);

const setupRivetkitRegistry = (
	registry: Registry,
	options?: {
		readonly serverless?: ServerlessOptions | undefined;
	},
) =>
	Effect.sync(() => {
		const rivetkitRegistry = Rivetkit.setup({
			use: Object.fromEntries(registry.rivetkitActors),
			...registry.options,
			logging: { baseLogger: registry.baseLogger },
			/**
			 * Effect owns SIGINT/SIGTERM: `NodeRuntime.runMain` interrupts, the
			 * scope closes, and `startRivetkitRegistry`'s finalizer runs
			 * `shutdown()`.
			 */
			shutdown: { disableSignalHandlers: true },
			...(options?.serverless === undefined
				? {}
				: { serverless: options.serverless }),
		});
		registry.rivetkitRegistry.current = rivetkitRegistry;
		return rivetkitRegistry;
	});

/**
 * Starts the envoy and waits until it registers with the engine. Closing the
 * surrounding scope shuts the registry down and sleeps its actors. A rejected
 * start is a defect.
 */
const startRivetkitRegistry = (
	rivetkitRegistry: Rivetkit.Registry<Rivetkit.RegistryActors>,
): Effect.Effect<void, never, Scope.Scope> =>
	Effect.gen(function* () {
		/**
		 * Added before the start, not via `acquireRelease`, so the start stays
		 * interruptible and a failed or interrupted start also shuts down.
		 */
		yield* Effect.addFinalizer(() =>
			Effect.promise(() => rivetkitRegistry.shutdown()),
		);
		yield* Effect.promise(() => rivetkitRegistry.startAndWait());
	});

/**
 * Serves the actors registered by `actorsLayer`.
 *
 * @remarks
 * Provide this with {@link layer|`Registry.layer()`} at the application root.
 * Building the returned layer evaluates the actor layer, collects its registered
 * actors, and starts the underlying RivetKit runtime using the options from
 * {@link layer|`Registry.layer()`}. The layer finishes building once the
 * envoy registers with the engine; a failed start is a defect. Launch the
 * resulting layer with {@link Layer.launch|`Layer.launch()`} to keep the
 * server process alive. Closing its scope (for example on SIGINT under
 * `NodeRuntime.runMain`) shuts the registry down and sleeps its actors.
 *
 * @param actorsLayer - A layer containing one or more
 * {@link Actor.Actor.toLayer|`Actor.toLayer()`} registrations.
 *
 * @returns A side-effecting startup layer that requires the
 * {@link Registry|`Registry.Registry`} service and the services required by
 * `actorsLayer`.
 *
 * @example
 * ```ts
 * import { Registry } from "@rivetkit/effect"
 * import { Layer } from "effect"
 *
 * const ActorsLayer = Layer.mergeAll(ChatRoomLive, ModeratorLive)
 *
 * const MainLayer = Registry.serve(ActorsLayer).pipe(
 *   Layer.provide(Registry.layer({ endpoint: "http://127.0.0.1:6420" }))
 * )
 *
 * Layer.launch(MainLayer).pipe(NodeRuntime.runMain);
 * ```
 */
export const serve = <E, R>(
	actorsLayer: Layer.Layer<never, E, R>,
): Layer.Layer<never, E, R | Registry> =>
	Layer.effectDiscard(
		Effect.gen(function* () {
			const registry = yield* Registry;
			yield* Layer.build(
				actorsLayer.pipe(
					Layer.provideMerge(
						RivetLogger.layerFromPino(registry.baseLogger),
					),
				),
			);
			const rivetkitRegistry = yield* setupRivetkitRegistry(registry);
			yield* startRivetkitRegistry(rivetkitRegistry);
		}),
	);

/**
 * Answers with a route of the RivetKit registry. Before `serve` makes that
 * registry, a registry that has not started gives RivetKit's own `503`.
 */
const answerWithRivetkitRoute = (
	route: (
		routes: Rivetkit.Registry<Rivetkit.RegistryActors>["routes"],
	) => Promise<Response>,
): Effect.Effect<HttpServerResponse.HttpServerResponse, never, Registry> =>
	Effect.gen(function* () {
		const registry = yield* Registry;
		const rivetkitRegistry =
			registry.rivetkitRegistry.current ?? Rivetkit.setup({ use: {} });
		const response = yield* Effect.promise(() =>
			route(rivetkitRegistry.routes),
		);
		return HttpServerResponse.fromWeb(response);
	});

/**
 * Answers a request with the Prometheus metrics of the registry, as
 * `registry.routes.prometheusMetrics()` of RivetKit does. Before
 * {@link serve|`Registry.serve()`} starts the registry, it answers `503`.
 *
 * Serve it on a private port. A cluster reads the metrics of a node from
 * `GET /metrics` on `127.0.0.1` at `RIVETKIT_METRICS_PORT` (default: 9464).
 *
 * @example
 * ```ts
 * import { createServer } from "node:http"
 * import { NodeHttpServer } from "@effect/platform-node"
 * import { Registry } from "@rivetkit/effect"
 * import { Layer } from "effect"
 * import { HttpRouter } from "effect/http"
 *
 * const MetricsServer = HttpRouter.add(
 *   "GET",
 *   "/metrics",
 *   Registry.prometheusMetrics,
 * ).pipe(
 *   HttpRouter.serve,
 *   Layer.provide(
 *     NodeHttpServer.layer(createServer, { host: "127.0.0.1", port: 9464 }),
 *   ),
 * )
 * ```
 */
export const prometheusMetrics: Effect.Effect<
	HttpServerResponse.HttpServerResponse,
	never,
	Registry
> = answerWithRivetkitRoute((routes) => routes.prometheusMetrics());

/**
 * Answers a request with the health of the registry as JSON, as
 * `registry.routes.health()` of RivetKit does, for probes. Before
 * {@link serve|`Registry.serve()`} starts the registry, it answers `503`
 * with `status: "not_started"`.
 *
 * @example
 * ```ts
 * import { Registry } from "@rivetkit/effect"
 * import { HttpRouter } from "effect/http"
 *
 * const HealthRoute = HttpRouter.add("GET", "/health", Registry.health)
 * ```
 */
export const health: Effect.Effect<
	HttpServerResponse.HttpServerResponse,
	never,
	Registry
> = answerWithRivetkitRoute((routes) => routes.health());

/**
 * In-process test runtime. Starts the rivetkit registry against the
 * configured engine, waits until its envoy registers, and provides
 * `Client` from the same Layer so consumers don't need to wire
 * `Client.layer` separately. Closing the layer's scope disposes the
 * client and shuts the registry down.
 */
export const test: Layer.Layer<Client.Client, never, Registry> = Layer.effect(
	Client.Client,
	Effect.gen(function* () {
		const registry = yield* Registry;
		const rivetkitRegistry = yield* setupRivetkitRegistry(registry);
		rivetkitRegistry.config.test = {
			...rivetkitRegistry.config.test,
			enabled: true,
		};
		rivetkitRegistry.config.noWelcome = true;
		/**
		 * Starts a local engine when the caller set neither `endpoint` nor
		 * `startEngine`, so `Registry.test` works without an engine.
		 */
		if (
			registry.options.endpoint === undefined &&
			registry.options.startEngine === undefined
		) {
			rivetkitRegistry.config.startEngine = true;
		}
		yield* startRivetkitRegistry(rivetkitRegistry);

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
	Effect.Effect<
		HttpServerResponse.HttpServerResponse,
		HttpServerError.HttpServerError,
		HttpServerRequest.HttpServerRequest
	>
> =>
	Effect.map(
		setupRivetkitRegistry(registry, {
			serverless: options,
		}),
		(rivetkitRegistry) =>
			HttpEffect.fromWebHandler((request) =>
				rivetkitRegistry.handler(request),
			),
	);

/**
 * @deprecated Serverless is disabled in RivetKit v3, so every request fails.
 * Use {@link serve|`Registry.serve()`} with `Layer.launch` instead.
 * See https://github.com/rivet-dev/rivet/blob/dcbfd6bc0d4f0b061c9cd2609472460ddccfa9a3/rivetkit-typescript/packages/rivetkit/src/registry/index.ts#L46-L49
 */
export type ToHttpEffectOptions = ServerlessOptions;

/**
 * Builds a scoped Effect HTTP handler from a registry layer.
 *
 * The registry layer is built once in the surrounding scope. Registered Rivet
 * Actors are materialized into a single underlying RivetKit registry, and each
 * request is delegated to that registry's serverless handler.
 *
 * @deprecated Serverless is disabled in RivetKit v3, so every request fails.
 * Use {@link serve|`Registry.serve()`} with `Layer.launch` instead.
 * See https://github.com/rivet-dev/rivet/blob/dcbfd6bc0d4f0b061c9cd2609472460ddccfa9a3/rivetkit-typescript/packages/rivetkit/src/registry/index.ts#L46-L49
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
	const httpEffect = yield* makeHttpEffect(
		Context.get(context, Registry),
		options,
	);
	// @effect-diagnostics-next-line returnEffectInGen:off
	return httpEffect.pipe(Effect.provide(context));
});

/**
 * @deprecated Serverless is disabled in RivetKit v3, so every request fails.
 * Use {@link serve|`Registry.serve()`} with `Layer.launch` instead.
 * See https://github.com/rivet-dev/rivet/blob/dcbfd6bc0d4f0b061c9cd2609472460ddccfa9a3/rivetkit-typescript/packages/rivetkit/src/registry/index.ts#L46-L49
 */
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
 *
 * @deprecated Serverless is disabled in RivetKit v3, so every request fails.
 * Use {@link serve|`Registry.serve()`} with `Layer.launch` instead.
 * See https://github.com/rivet-dev/rivet/blob/dcbfd6bc0d4f0b061c9cd2609472460ddccfa9a3/rivetkit-typescript/packages/rivetkit/src/registry/index.ts#L46-L49
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
			Effect.map(
				makeHttpEffect(
					Context.get(context, Registry),
					serverlessOptions,
				),
				(httpEffect) => httpEffect.pipe(Effect.provide(context)),
			),
		middleware,
		memoMap,
	});
};
