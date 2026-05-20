import { Context, Effect, Layer } from "effect";
import { HttpEffect, HttpMiddleware } from "effect/unstable/http";
import * as Rivetkit from "rivetkit";
import * as Client from "./Client";

const TypeId = "~@rivetkit/effect/Registry";

export type Options = Pick<
	Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>,
	"endpoint" | "token" | "namespace"
>;

export interface Registry {
	readonly [TypeId]: typeof TypeId;

	readonly options: Options;

	readonly rivetkitActors: Map<string, Rivetkit.AnyActorDefinition>;
}

export const Registry: Context.Service<Registry, Registry> =
	Context.Service<Registry>("@rivetkit/effect/Registry");

const make = (options: Options = {}): Registry => {
	return Registry.of({
		[TypeId]: TypeId,
		options,
		rivetkitActors: new Map(),
	});
};

export const layer = (options: Options = {}): Layer.Layer<Registry> =>
	Layer.succeed(Registry, make(options));

const setupRivetkitRegistry = (
	registry: Registry,
	options?: {
		readonly serverless?:
			| Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>["serverless"]
			| undefined;
	},
) =>
	Rivetkit.setup({
		use: Object.fromEntries(registry.rivetkitActors),
		...registry.options,
		...(options?.serverless === undefined
			? {}
			: { serverless: options.serverless }),
	});

/**
 * Run the registered actors against the configured engine. Reads
 * the collected entries, materializes the underlying rivetkit
 * registry, and starts it.
 */
export const serve: Layer.Layer<never, never, Registry> = Layer.effectDiscard(
	Effect.gen(function* () {
		const registry = yield* Registry;
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
		});
	}),
);

export type ToWebHandlerOptions = {
	readonly serverless?:
		| Rivetkit.RegistryConfigInput<Rivetkit.RegistryActors>["serverless"]
		| undefined;
	readonly middleware?: HttpMiddleware.HttpMiddleware | undefined;
	readonly memoMap?: Layer.MemoMap | undefined;
};

export const toWebHandler = <E>(
	registryLayer: Layer.Layer<Registry, E>,
	options?: ToWebHandlerOptions,
) =>
	HttpEffect.toWebHandlerLayerWith(registryLayer, {
		toHandler: (context) =>
			Effect.sync(() => {
				const registry = Context.get(context, Registry);
				const rivetkitRegistry = setupRivetkitRegistry(registry, {
					serverless: options?.serverless,
				});
				return HttpEffect.fromWebHandler((request) =>
					rivetkitRegistry.handler(request),
				);
			}),
		middleware: options?.middleware,
		memoMap: options?.memoMap,
	});
