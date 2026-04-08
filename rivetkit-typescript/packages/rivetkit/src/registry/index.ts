import { Runtime } from "../../runtime";
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";

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

		// Start the local manager or engine before /api/rivet is hit so clients can
		// reach the public endpoint preemptively. This waits one tick because some
		// integrations mutate registry config immediately after setup() returns.
		if (config.serverless?.spawnEngine || config.serveManager) {
			setTimeout(() => {
				const parsedConfig = this.parseConfig();

				if (
					parsedConfig.serverless.spawnEngine ||
					parsedConfig.serveManager
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
	 * This is the simplest way to run RivetKit. It starts a local manager
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

		// Force serveManager when there's no remote endpoint so the
		// manager server starts and serves the API + static files.
		// When an endpoint IS configured, the config transform handles
		// the mode (serveManager defaults to false, spawnEngine may be
		// true, etc.) and we just start the envoy.
		if (this.#config.serveManager === undefined) {
			const hasEndpoint = !!(
				this.#config.endpoint ||
				(typeof process !== "undefined" &&
					(process.env.RIVET_ENGINE || process.env.RIVET_ENDPOINT))
			);
			const willSpawnEngine = !!this.#config.serverless?.spawnEngine;
			if (!hasEndpoint && !willSpawnEngine) {
				this.#config.serveManager = true;
			}
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

export type { RegistryActors, RegistryConfig };
export { RegistryConfigSchema };
