import invariant from "invariant";
import { createClientWithDriver } from "@/client/client";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import { chooseDefaultDriver } from "@/drivers/default";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import {
	configureInspectorAccessToken,
	getInspectorUrl,
	isInspectorEnabled,
} from "@/inspector/utils";
import pkg from "../../package.json" with { type: "json" };
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config/registry";
import { type BaseConfig, type DriverConfig } from "./config/base";
import {
	type RunnerConfig,
	type RunnerConfigInput,
	RunnerConfigSchema,
} from "./config/runner";
import {
	type ServerlessConfig,
	type ServerlessConfigInput,
	ServerlessConfigSchema,
} from "./config/serverless";
import {
	type LegacyRunnerConfig,
	type LegacyRunnerConfigInput,
	LegacyRunnerConfigSchema,
} from "./config/legacy-runner";
import { logger } from "./log";
import { buildServerlessRouter } from "@/serverless/router";
import { buildManagerRouter } from "@/manager/router";
import { crossPlatformServe, findFreePort } from "./serve";
import { GetUpgradeWebSocket } from "@/utils";
import { configureServerlessRunner } from "@/serverless/configure";
import type { Client } from "@/client/client";
import { createClient } from "@/client/mod";
import { isDev } from "@/utils/env-vars";

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
	#config: RegistryConfig;

	public get config(): RegistryConfig {
		return this.#config;
	}

	constructor(config: RegistryConfig) {
		this.#config = config;
	}

	public handler(inputConfig: ServerlessConfigInput = {}): ServerlessHandler {
		const config = ServerlessConfigSchema.parse(inputConfig);

		const { driver } = this.#start(config, { serverless: config });

		const { router } = buildServerlessRouter(driver, this.#config, config);
		return { fetch: router.fetch };
	}

	public startRunner(inputConfig: RunnerConfigInput = {}) {
		const config = RunnerConfigSchema.parse(inputConfig);

		this.#start(config, { runner: config });
	}

	#start(
		baseConfig: BaseConfig,
		config: { serverless?: ServerlessConfig; runner?: RunnerConfig },
	): { driver: DriverConfig } {
		// Promise for any async operations we need to wait to complete
		const readyPromises: Promise<unknown>[] = [];

		// Configure logger
		if (baseConfig.logging?.baseLogger) {
			// Use provided base logger
			configureBaseLogger(baseConfig.logging.baseLogger);
		} else {
			// Configure default logger with log level from config getPinoLevel
			// will handle env variable priority
			configureDefaultLogger(baseConfig.logging?.level);
		}

		// Handle spawnEngine before choosing driver
		// Start engine
		invariant(
			!(config.serverless?.spawnEngine && baseConfig.serveManager),
			"cannot specify spawnEngine and serveManager together",
		);

		if (config.serverless?.spawnEngine) {
			logger().debug({
				msg: "run engine requested",
				version: config.serverless.engineVersion,
			});

			// Set config to point to the engine
			invariant(
				baseConfig.endpoint === undefined,
				"cannot specify endpoint with spawnEngine",
			);
			baseConfig.endpoint = ENGINE_ENDPOINT;

			// Start the engine
			const engineProcessPromise = ensureEngineProcess({
				version: config.serverless.engineVersion,
			});

			// Chain ready promise
			readyPromises.push(engineProcessPromise);
		}

		// Choose the driver based on configuration (after endpoint may have been set by spawnEngine)
		const driver = chooseDefaultDriver(baseConfig);

		// Create manager driver (always needed for actor driver + inline client)
		const managerDriver = driver.manager(this.#config, baseConfig);
		configureInspectorAccessToken(baseConfig, managerDriver);

		if (baseConfig.serveManager) {
			// Configure getUpgradeWebSocket lazily so we can assign it in crossPlatformServe
			let upgradeWebSocket: any;
			const getUpgradeWebSocket: GetUpgradeWebSocket = () =>
				upgradeWebSocket;
			managerDriver.setGetUpgradeWebSocket(getUpgradeWebSocket);

			// Build router
			const { router: managerRouter } = buildManagerRouter(
				this.#config,
				baseConfig,
				managerDriver,
				getUpgradeWebSocket,
			);

			// Serve manager
			const serverPromise = (async () => {
				const managerPort = await findFreePort(baseConfig.managerPort);
				baseConfig.managerPort = managerPort;

				const out = await crossPlatformServe(baseConfig, managerRouter);
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
			if (config.runner && driver.autoStartActorDriver) {
				logger().debug("starting actor driver");
				const inlineClient =
					createClientWithDriver<this>(managerDriver);
				driver.actor(
					this.#config,
					config.runner,
					managerDriver,
					inlineClient,
				);
			}

			// Log starting
			const driverLog = managerDriver.extraStartupLog?.() ?? {};
			logger().info({
				msg: "rivetkit ready",
				driver: driver.name,
				definitions: Object.keys(this.#config.use).length,
				...driverLog,
			});
			const inspectorUrl = getInspectorUrl(baseConfig);
			if (
				inspectorUrl &&
				isInspectorEnabled(baseConfig, "manager") &&
				managerDriver.inspector
			) {
				logger().info({
					msg: "inspector ready",
					url: inspectorUrl,
				});
			}

			// Print welcome information
			if (!baseConfig.noWelcome) {
				console.log();
				console.log(
					`  RivetKit ${pkg.version} (${driver.displayName})`,
				);
				// Only show endpoint if manager is running or engine is spawned
				const shouldShowEndpoint = baseConfig.serveManager || config.serverless?.spawnEngine;
				if (config.serverless?.advertiseEndpoint && shouldShowEndpoint) {
					console.log(
						`  - Endpoint:     ${config.serverless?.advertiseEndpoint}`,
					);
				}
				if (config.serverless?.spawnEngine) {
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
					isInspectorEnabled(baseConfig, "manager") &&
					managerDriver.inspector
				) {
					console.log(`  - Inspector:    ${inspectorUrl}`);
				}
				console.log();
			}

			// Configure serverless runner if enabled when actor driver is disabled
			if (config.serverless?.configureRunnerPool) {
				await configureServerlessRunner(config.serverless);
			}
		});

		return { driver };
	}

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
		inputConfig: LegacyRunnerConfigInput | undefined,
	): LegacyStartServerOutput<this> {
		// Convert legacy autoConfigureServerless to new configureRunnerPool format
		let configureRunnerPool: ServerlessConfigInput["configureRunnerPool"];
		if (config.autoConfigureServerless) {
			// Derive URL from overrideServerAddress or default to localhost
			const defaultUrl =
				config.overrideServerAddress ?? "http://localhost:8080";

			if (typeof config.autoConfigureServerless === "boolean") {
				// Legacy boolean: use default url
				configureRunnerPool = { url: defaultUrl };
			} else {
				// Legacy object: merge with url fallback
				configureRunnerPool = {
					...config.autoConfigureServerless,
					url: config.autoConfigureServerless.url ?? defaultUrl,
				};
			}
		}

		// Convert legacy config to serverless config
		const serverlessConfig: ServerlessConfigInput = {
			driver: config.driver as any,
			maxIncomingMessageSize: config.maxIncomingMessageSize,
			maxOutgoingMessageSize: config.maxOutgoingMessageSize,
			noWelcome: config.noWelcome,
			logging: config.logging,
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
			managerBasePath: config.basePath,
			managerPort: 8080, // Legacy serverless used port 8080
			inspector: config.inspector,
			// Map legacy fields to new fields
			spawnEngine: config.runEngine,
			engineVersion: config.runEngineVersion,
			advertiseEndpoint: config.overrideServerAddress,
			configureRunnerPool,
			// totalSlots is not used in serverless - it's managed by the engine
		};

		const handler = this.handler(serverlessConfig);

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
			fetch: handler.fetch,
		};
	}

	#legacyStartNormal(
		config: LegacyRunnerConfig,
	): LegacyStartServerOutput<this> {
		// Convert legacy config to runner config
		const runnerConfig: RunnerConfigInput = {
			driver: config.driver as any,
			maxIncomingMessageSize: config.maxIncomingMessageSize,
			maxOutgoingMessageSize: config.maxOutgoingMessageSize,
			noWelcome: config.noWelcome,
			logging: config.logging,
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
			managerBasePath: config.basePath,
			managerPort: config.defaultServerPort,
			inspector: config.inspector,
			// Map legacy serveManager logic
			// disableDefaultServer=true means serveManager=false
			serveManager: !config.disableDefaultServer,
		};

		// Start the runner
		this.startRunner(runnerConfig);

		// Create client for the legacy return value
		const client = createClient<this>({
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			headers: config.headers,
		});

		// For normal runner, we need to build a manager router to get the fetch handler
		// Parse the config to get the fully resolved config
		const parsedConfig = RunnerConfigSchema.parse(runnerConfig);
		const driver = chooseDefaultDriver(parsedConfig);
		const managerDriver = driver.manager(this.#config, parsedConfig);

		// Configure getUpgradeWebSocket as undefined for this legacy path
		// since it's only used when actually serving
		const { router } = buildManagerRouter(
			this.#config,
			parsedConfig,
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
