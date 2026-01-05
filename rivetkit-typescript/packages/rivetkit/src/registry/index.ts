import invariant from "invariant";
import type { Client } from "@/client/client";
import { createClientWithDriver } from "@/client/client";
import { createClient } from "@/client/mod";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import { chooseDefaultDriver } from "@/drivers/default";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import {
	configureInspectorAccessToken,
	getInspectorUrl,
	isInspectorEnabled,
} from "@/inspector/utils";
import { buildManagerRouter } from "@/manager/router";
import { configureServerlessRunner } from "@/serverless/configure";
import { buildServerlessRouter } from "@/serverless/router";
import type { GetUpgradeWebSocket } from "@/utils";
import { isDev } from "@/utils/env-vars";
import pkg from "../../package.json" with { type: "json" };
import {
	type DriverConfig,
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
import { logger } from "./log";
import { crossPlatformServe, findFreePort } from "./serve";

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

/**
 * Defines what type of server is being started. Used internally for
 * Registry.#start
 **/
type StartKind = "serverless" | "runner";

export class Registry<A extends RegistryActors> {
	#config: RegistryConfig;

	/**
	 * Cached serverless state. Subsequent calls to `handler()` will use this.
	 */
	#serverlessState: ServerlessHandler | null = null;

	public get config(): RegistryConfig {
		return this.#config;
	}

	constructor(config: RegistryConfig) {
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
		const { fetch } = this.#ensureServerlessInitialized();
		return fetch(request);
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
		this.#start("runner");
	}

	/** Lazily initializes serverless state on first request, caches for subsequent calls. */
	#ensureServerlessInitialized(): { fetch: FetchHandler } {
		if (!this.#serverlessState) {
			const { driver } = this.#start("serverless");

			const { router } = buildServerlessRouter(driver, this.#config);
			this.#serverlessState = { fetch: router.fetch.bind(router) };
		}
		return this.#serverlessState;
	}

	#start(kind: StartKind): { driver: DriverConfig } {
		const config = this.#config;

		// Promise for any async operations we need to wait to complete
		const readyPromises: Promise<unknown>[] = [];

		// Configure logger
		if (config.logging?.baseLogger) {
			// Use provided base logger
			configureBaseLogger(config.logging.baseLogger);
		} else {
			// Configure default logger with log level from config getPinoLevel
			// will handle env variable priority
			configureDefaultLogger(config.logging?.level);
		}

		// Handle spawnEngine before choosing driver
		// Start engine
		invariant(
			!(
				kind === "serverless" &&
				config.serverless.spawnEngine &&
				config.serveManager
			),
			"cannot specify spawnEngine and serveManager together",
		);

		if (kind === "serverless" && config.serverless.spawnEngine) {
			logger().debug({
				msg: "run engine requested",
				version: config.serverless.engineVersion,
			});

			// Set config to point to the engine
			invariant(
				config.endpoint === undefined,
				"cannot specify endpoint with spawnEngine",
			);
			config.endpoint = ENGINE_ENDPOINT;

			// Start the engine
			const engineProcessPromise = ensureEngineProcess({
				version: config.serverless.engineVersion,
			});

			// Chain ready promise
			readyPromises.push(engineProcessPromise);
		}

		// Choose the driver based on configuration (after endpoint may have been set by spawnEngine)
		const driver = chooseDefaultDriver(config);

		// Create manager driver (always needed for actor driver + inline client)
		const managerDriver = driver.manager(this.#config);
		configureInspectorAccessToken(config, managerDriver);

		if (config.serveManager) {
			// Configure getUpgradeWebSocket lazily so we can assign it in crossPlatformServe
			let upgradeWebSocket: any;
			const getUpgradeWebSocket: GetUpgradeWebSocket = () =>
				upgradeWebSocket;
			managerDriver.setGetUpgradeWebSocket(getUpgradeWebSocket);

			// Build router
			const { router: managerRouter } = buildManagerRouter(
				this.#config,
				managerDriver,
				getUpgradeWebSocket,
			);

			// Serve manager
			const serverPromise = (async () => {
				const managerPort = await findFreePort(config.managerPort);
				config.managerPort = managerPort;

				const out = await crossPlatformServe(config, managerRouter);
				upgradeWebSocket = out.upgradeWebSocket;
			})();
			readyPromises.push(serverPromise);
		}

		// Log and print welcome after all ready promises complete
		// biome-ignore lint/nursery/noFloatingPromises: bg promise
		Promise.all(readyPromises).then(async () => {
			// Auto-start actor driver for drivers that require it.
			//
			// This is only enabled for runner config since serverless will
			// auto-start the actor driver on `GET /start`.
			if (
				kind === "runner" &&
				config.runner &&
				driver.autoStartActorDriver
			) {
				logger().debug("starting actor driver");
				const inlineClient =
					createClientWithDriver<this>(managerDriver);
				driver.actor(this.#config, managerDriver, inlineClient);
			}

			// Log starting
			const driverLog = managerDriver.extraStartupLog?.() ?? {};
			logger().info({
				msg: "rivetkit ready",
				driver: driver.name,
				definitions: Object.keys(this.#config.use).length,
				...driverLog,
			});
			const inspectorUrl = getInspectorUrl(config);
			if (
				inspectorUrl &&
				isInspectorEnabled(config, "manager") &&
				managerDriver.inspector
			) {
				logger().info({
					msg: "inspector ready",
					url: inspectorUrl,
				});
			}

			// Print welcome information
			if (!config.noWelcome) {
				console.log();
				console.log(
					`  RivetKit ${pkg.version} (${driver.displayName})`,
				);
				// Only show endpoint if manager is running or engine is spawned
				const shouldShowEndpoint =
					config.serveManager ||
					(kind === "serverless" && config.serverless.spawnEngine);
				if (
					kind === "serverless" &&
					config.serverless.advertiseEndpoint &&
					shouldShowEndpoint
				) {
					console.log(
						`  - Endpoint:     ${config.serverless.advertiseEndpoint}`,
					);
				}
				if (kind === "serverless" && config.serverless.spawnEngine) {
					const padding = " ".repeat(
						Math.max(0, 13 - "Engine".length),
					);
					console.log(
						`  - Engine:${padding}v${config.serverless.engineVersion}`,
					);
				}
				const displayInfo = managerDriver.displayInformation();
				for (const [k, v] of Object.entries(displayInfo.properties)) {
					const padding = " ".repeat(Math.max(0, 13 - k.length));
					console.log(`  - ${k}:${padding}${v}`);
				}
				if (
					inspectorUrl &&
					isInspectorEnabled(config, "manager") &&
					managerDriver.inspector
				) {
					console.log(`  - Inspector:    ${inspectorUrl}`);
				}
				console.log();
			}

			// Configure serverless runner if enabled when actor driver is disabled
			if (
				kind === "serverless" &&
				config.serverless.configureRunnerPool
			) {
				await configureServerlessRunner(config);
			}
		});

		return { driver };
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
		this.startRunner();

		// Create client for the legacy return value
		const client = createClient<this>({
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
		});

		// For normal runner, we need to build a manager router to get the fetch handler
		const driver = chooseDefaultDriver(this.#config);
		const managerDriver = driver.manager(this.#config);

		// Configure getUpgradeWebSocket as undefined for this legacy path
		// since it's only used when actually serving
		const { router } = buildManagerRouter(
			this.#config,
			managerDriver,
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
	const config = RegistryConfigSchema.parse(input);
	return new Registry(config);
}

export type { RegistryConfig, RegistryActors };
export { RegistryConfigSchema };
