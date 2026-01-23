import type { Client } from "@/client/client";
import { createClient } from "@/client/mod";
import { isDev } from "@/utils/env-vars";
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import {
	type LegacyRunnerConfig,
	type LegacyRunnerConfigInput,
	LegacyRunnerConfigSchema,
} from "./config/legacy-runner";
import { Runtime } from "../../runtime";

export type FetchHandler = (
	request: Request,
	...args: any
) => Response | Promise<Response>;

export interface ServerlessHandler {
	fetch: FetchHandler;
}

export interface LegacyStartServerOutput<A extends Registry<any>> {
	/** Client to communicate with the actors. */
	client: Client<A>;
	/** Fetch handler to manually route requests to the Rivet manager API. */
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

		// Auto-prepare on next tick (gives time for sync config modification).
		// Skip in edge runtimes (Convex, Cloudflare Workers) where setTimeout
		// throws at module load time. We detect this by checking for process.versions.node
		// which only exists in Node.js/Bun/Deno.
		const isNodeLike =
			typeof process !== "undefined" && process.versions?.node;
		if (isNodeLike && typeof setTimeout !== "undefined") {
			setTimeout(() => {
				// biome-ignore lint/nursery/noFloatingPromises: fire-and-forget auto-prepare
				this.#ensureRuntime();
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
	 * Starts an actor runner for standalone server deployments.
	 */
	public startRunner() {
		// biome-ignore lint/nursery/noFloatingPromises: bg task
		this.#ensureRuntime().then((runtime) => runtime.startRunner());
	}

	// MARK: Legacy
	/**
	 * Runs the registry for a server.
	 *
	 * @deprecated Use {@link Registry.startRunner} for long-running servers or {@link Registry.handler} for serverless deployments.
	 */
	public start(
		inputConfig?: LegacyRunnerConfigInput,
	): LegacyStartServerOutput<this> {
		const config = LegacyRunnerConfigSchema.parse(inputConfig);

		// Validate autoConfigureServerless is only used with serverless runner
		if (
			config.autoConfigureServerless &&
			config.runnerKind !== "serverless"
		) {
			throw new Error(
				"autoConfigureServerless can only be configured when runnerKind is 'serverless'",
			);
		}

		// Auto-configure serverless runner if not in prod
		const isDevEnv = isDev();
		if (isDevEnv && config.runnerKind === "serverless") {
			if (inputConfig?.runEngine === undefined) config.runEngine = true;
			if (inputConfig?.autoConfigureServerless === undefined)
				config.autoConfigureServerless = true;
		}

		// Convert to new config format and call appropriate handler
		if (config.runnerKind === "serverless") {
			return this.#legacyStartServerless(config, inputConfig);
		} else {
			return this.#legacyStartNormal(config);
		}
	}

	#legacyStartServerless(
		config: LegacyRunnerConfig,
		_inputConfig: LegacyRunnerConfigInput | undefined,
	): LegacyStartServerOutput<this> {
		// Create client for the legacy return value
		// For serverless, we don't have an endpoint until /start is called,
		// so we create a placeholder client
		const client = createClient<this>({
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
		});

		return {
			client,
			fetch: this.handler.bind(this),
		};
	}

	#legacyStartNormal(
		config: LegacyRunnerConfig,
	): LegacyStartServerOutput<this> {
		// Start the runner (fire-and-forget to maintain sync API)
		// biome-ignore lint/nursery/noFloatingPromises: legacy sync API
		this.#ensureRuntime().then((runtime) => runtime.startRunner());

		// Create client for the legacy return value
		const client = createClient<this>({
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
		});

		return {
			client,
			fetch: this.handler.bind(this),
		};
	}
}

export function setup<A extends RegistryActors>(
	input: RegistryConfigInput<A>,
): Registry<A> {
	return new Registry(input);
}

export type { RegistryConfig, RegistryActors };
export { RegistryConfigSchema };
