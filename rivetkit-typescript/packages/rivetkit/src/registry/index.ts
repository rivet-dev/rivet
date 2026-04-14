import { Runtime } from "../../runtime";
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import { logger } from "./log";

export type FetchHandler = (
	request: Request,
	...args: any
) => Response | Promise<Response>;

export interface ServerlessHandler {
	fetch: FetchHandler;
}

export class Registry<A extends RegistryActors> {
	#config: RegistryConfigInput<A>;

	get config(): RegistryConfigInput<A> {
		return this.#config;
	}

	parseConfig(): RegistryConfig {
		return RegistryConfigSchema.parse(this.#config);
	}

	// Shared runtime instance
	#runtime?: Runtime<A>;
	#runtimePromise?: Promise<Runtime<A>>;

	constructor(config: RegistryConfigInput<A>) {
		this.#config = config;

		// Start the local runtime or engine before /api/rivet is hit so
		// clients can reach the public endpoint preemptively. This waits
		// one tick because some integrations mutate registry config
		// immediately after setup() returns.
		//
		// Check both the canonical `runtime.spawnEngine` location and
		// the legacy `serverless.spawnEngine` so either config shape
		// triggers the preemptive runtime.
		const willSpawnEngine = !!(
			config.runtime?.spawnEngine ?? config.serverless?.spawnEngine
		);
		if (willSpawnEngine || config.serveHttp) {
			setTimeout(() => {
				const parsedConfig = this.parseConfig();

				if (
					parsedConfig.serverless.spawnEngine ||
					parsedConfig.serveHttp
				) {
					// biome-ignore lint/nursery/noFloatingPromises: fire-and-forget auto-prepare
					this.#ensureRuntime();
				}
			}, 0);
		}
	}

	/** Creates runtime if not already created. Idempotent. */
	#ensureRuntime(): Promise<Runtime<A>> {
		if (!this.#runtimePromise) {
			this.#runtimePromise = Runtime.create(this);
			// biome-ignore lint/nursery/noFloatingPromises: bg task
			this.#runtimePromise.then((rt) => {
				this.#runtime = rt;
			});
		}
		return this.#runtimePromise;
	}

	/**
	 * Handle an incoming HTTP request for serverless deployments.
	 *
	 * @example
	 * ```ts
	 * const app = new Hono();
	 * app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
	 * export default app;
	 * ```
	 */
	public async handler(request: Request): Promise<Response> {
		const runtime = await this.#ensureRuntime();
		runtime.startServerless();
		return await runtime.handleServerlessRequest(request);
	}

	/**
	 * Returns a fetch handler for serverless deployments.
	 *
	 * @example
	 * ```ts
	 * export default registry.serve();
	 * ```
	 */
	public serve(): ServerlessHandler {
		return { fetch: this.handler.bind(this) };
	}

	/**
	 * Starts an actor envoy for standalone server deployments.
	 */
	public startEnvoy() {
		// biome-ignore lint/nursery/noFloatingPromises: bg task
		this.#ensureRuntime().then((runtime) => runtime.startEnvoy());
	}

	/**
	 * Starts the server, serving both the actor API and static files.
	 *
	 * This is the simplest way to run RivetKit. It starts a local runtime
	 * server, serves static files from the configured `publicDir` (default
	 * `"public"`), and starts the actor envoy.
	 *
	 * When an endpoint is configured (via config or RIVET_ENDPOINT env var),
	 * operates in serverless mode connected to the remote engine instead.
	 *
	 * @example
	 * ```ts
	 * const registry = setup({ use: { counter } });
	 * registry.start();
	 * ```
	 */
	public start() {
		// Default publicDir to "public" if not explicitly set
		if (this.#config.publicDir === undefined) {
			this.#config.publicDir = "public";
		}

		// Resolve the runtime mode + spawn-engine decision via this matrix:
		//
		//                 | Default | NODE_ENV=prod | RIVET_ENDPOINT!=null | mode=envoy override
		//   spawn_engine  |   y     | error if no   |         n            |        n
		//                 |         | RIVET_ENDPOINT|                      |
		//   mode          | envoy   | serverless    |     serverless       |      envoy
		//
		// The user can override the mode explicitly by passing
		// `runtime: { mode: "envoy" }` (or `"serverless"`) to `setup()`.
		// `start()` drives the envoy path today; serverless deployments
		// still call `registry.handler()` directly.
		//
		// TODO (pending upstream refactors):
		//   - dispatch `start()` to startServerless when mode=serverless
		//   - drop "runner" terminology
		//   - migrate existing `serverless.spawnEngine` callers to
		//     top-level `runtime.spawnEngine` (field already exists)
		const runtimeCfg = this.#config.runtime;
		const hasEndpoint = !!(
			this.#config.endpoint ||
			(typeof process !== "undefined" &&
				(process.env.RIVET_ENGINE || process.env.RIVET_ENDPOINT))
		);
		const isProduction =
			typeof process !== "undefined" &&
			process.env.NODE_ENV === "production";

		// Resolve mode: explicit override wins, otherwise fall back to the
		// matrix — envoy by default, only NODE_ENV=production flips the
		// auto-default to serverless. RIVET_ENDPOINT alone does NOT force
		// serverless; envoy mode can still connect to a remote engine.
		const resolvedMode: "envoy" | "serverless" =
			runtimeCfg?.mode ?? (isProduction ? "serverless" : "envoy");

		// Resolve spawnEngine. Explicit `runtime.spawnEngine` and the
		// legacy `serverless.spawnEngine` both win when set. Otherwise the
		// matrix decides. In envoy mode without an endpoint we auto-spawn
		// the local engine; in serverless or with an endpoint we don't.
		const explicitSpawn =
			runtimeCfg?.spawnEngine ?? this.#config.serverless?.spawnEngine;
		if (explicitSpawn === undefined) {
			if (resolvedMode === "serverless" && isProduction && !hasEndpoint) {
				throw new Error(
					"rivetkit: NODE_ENV=production requires RIVET_ENDPOINT " +
						"(or an explicit `endpoint` config) to connect to a " +
						"hosted engine.",
				);
			}
			if (resolvedMode === "envoy" && !hasEndpoint) {
				// Envoy-mode dev default: spawn the engine locally so the
				// registry boots with zero config. The user app runs no
				// HTTP server — the engine on 6420 is the public surface.
				// Write to the canonical `runtime.spawnEngine` location;
				// the schema transform normalizes it into the legacy
				// `serverless.spawnEngine` field that downstream code
				// still reads.
				this.#config.runtime = {
					...(this.#config.runtime ?? { mode: "envoy" as const }),
					spawnEngine: true,
				};
			}
			// All other cells leave spawnEngine undefined → schema default
			// resolves to `false` (connect to remote without spawning).
		}

		// `start()` drives the envoy path. When the user explicitly picks
		// `mode: "serverless"` but still calls `start()`, log a hint so the
		// mis-wiring is obvious — they should use `registry.handler()`.
		if (resolvedMode === "serverless") {
			logger().warn({
				msg: "registry.start() called with runtime.mode=serverless; " +
					"serverless deployments should use `registry.handler()` " +
					"to mount the /api/rivet/* fetch handler in your HTTP " +
					"server instead.",
			});
		}

		// biome-ignore lint/nursery/noFloatingPromises: fire-and-forget
		this.#ensureRuntime().then((runtime) => runtime.startEnvoy());
	}
}

export function setup<A extends RegistryActors>(
	input: RegistryConfigInput<A>,
): Registry<A> {
	return new Registry(input);
}

export type { RegistryConfig, RegistryActors };
export { RegistryConfigSchema };
