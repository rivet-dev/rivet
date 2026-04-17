import { Runtime } from "../../runtime";
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import { buildNativeRegistry } from "./native";

export type FetchHandler = (
	request: Request,
	...args: any
) => Response | Promise<Response>;

export interface ServerlessHandler {
	fetch: FetchHandler;
}

function removedLegacyRoutingError(method: string): Error {
	return new Error(
		`Registry.${method}() used the removed TypeScript routing/serverless stack. Use Registry.startEnvoy() and route traffic through the engine instead.`,
	);
}

export class Registry<A extends RegistryActors> {
	#config: RegistryConfigInput<A>;

	get config(): RegistryConfigInput<A> {
		return this.#config;
	}

	parseConfig(): RegistryConfig {
		return RegistryConfigSchema.parse(this.#config);
	}

	#runtimePromise?: Promise<Runtime<A>>;
	#nativeServePromise?: Promise<void>;

	constructor(config: RegistryConfigInput<A>) {
		this.#config = config;

		// Start the local engine before /api/rivet is hit so clients can
		// reach the endpoint preemptively. This waits one tick because some
		// integrations mutate registry config immediately after setup() returns.
		if (config.startEngine) {
			setTimeout(() => {
				const parsedConfig = this.parseConfig();

				if (parsedConfig.startEngine) {
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
		void request;
		throw removedLegacyRoutingError("handler");
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
		return {
			fetch: async (request) => {
				void request;
				throw removedLegacyRoutingError("serve");
			},
		};
	}

	/**
	 * Starts an actor envoy for standalone server deployments.
	 */
	public startEnvoy() {
		if (!this.#nativeServePromise) {
			this.#nativeServePromise = buildNativeRegistry(
				this.parseConfig(),
			).then(async ({ registry, serveConfig }) => {
				await registry.serve(serveConfig);
			});
		}
	}

	/**
	 * Starts the server, serving both the actor API and static files.
	 *
	 * This is the simplest way to run RivetKit. It starts a local runtime
	 * server, serves static files from the configured `staticDir` (default
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
		this.startEnvoy();
	}
}

export function setup<A extends RegistryActors>(
	input: RegistryConfigInput<A>,
): Registry<A> {
	return new Registry(input);
}

export type { RegistryConfig, RegistryActors };
export { RegistryConfigSchema };
