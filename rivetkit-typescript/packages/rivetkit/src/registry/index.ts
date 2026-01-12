import type { Client } from "@/client/client";
import { createClient } from "@/client/mod";
import { chooseDefaultDriver } from "@/drivers/default";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import { getInspectorUrl } from "@/inspector/utils";
import { buildManagerRouter } from "@/manager/router";
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

	// HACK: We need to be able to call `registry.handler` cheaply without
	// re-initializing the runtime every time. We lazily create the runtime and
	// store it here for future calls to `registry.handler`.
	#cachedServerlessRuntime?: Runtime<A>;

	constructor(config: RegistryConfigInput<A>) {
		this.#config = config;
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
	public handler(request: Request): Response | Promise<Response> {
		return this.#ensureServerlessInitialized().handler(request);
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

	/** Lazily initializes serverless state on first request, caches for subsequent calls. */
	#ensureServerlessInitialized(): Runtime<A> {
		if (!this.#cachedServerlessRuntime) {
			this.#cachedServerlessRuntime = new Runtime(this, "serverless");
		}
		return this.#cachedServerlessRuntime;
	}

	/**
	 * Starts an actor runner for standalone server deployments.
	 */
	public startRunner() {
		new Runtime(this, "runner");
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
		// Start the runner
		// Note: Legacy config is ignored - all config should now be passed to setup()
		const runtime = new Runtime(this, "runner");

		// Create client for the legacy return value
		const client = createClient<this>({
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
		});

		// Configure getUpgradeWebSocket as undefined for this legacy path
		// since it's only used when actually serving
		const { router } = buildManagerRouter(
			runtime.config,
			runtime.managerDriver,
			undefined, // getUpgradeWebSocket
		);

		return {
			client,
			fetch: router.fetch,
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
