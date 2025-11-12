import invariant from "invariant";
import { type Client, createClientWithDriver } from "@/client/client";
import type { ClientConfig } from "@/client/config";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import type { ActorDriver } from "@/driver-helpers/mod";
import { chooseDefaultDriver } from "@/drivers/default";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import { getInspectorUrl } from "@/inspector/utils";
import { createManagerRouter } from "@/manager/router";
import {
	getDatacenters,
	type RunnerConfigRequest,
	updateRunnerConfig,
} from "@/remote-manager-driver/api-endpoints";
import { getEndpoint } from "@/remote-manager-driver/api-utils";
import pkg from "../../package.json" with { type: "json" };
import {
	type RegistryActors,
	type RegistryConfig,
	type RegistryConfigInput,
	RegistryConfigSchema,
} from "./config";
import { logger } from "./log";
import {
	type DriverConfig,
	type RunnerConfig,
	type RunnerConfigInput,
	RunnerConfigSchema,
} from "./run-config";
import { crossPlatformServe } from "./serve";

export type ServerlessActorDriverBuilder = (
	updateConfig: (config: RunnerConfig) => void,
) => ActorDriver;

interface ServerOutput<A extends Registry<any>> {
	/** Client to communicate with the actors. */
	client: Client<A>;
	/** Fetch handler to manually route requests to the Rivet manager API. */
	fetch: (request: Request, ...args: any) => Response | Promise<Response>;
}

export class Registry<A extends RegistryActors> {
	#config: RegistryConfig;

	public get config(): RegistryConfig {
		return this.#config;
	}

	constructor(config: RegistryConfig) {
		this.#config = config;
	}

	/**
	 * Runs the registry for a server.
	 */
	public start(inputConfig?: RunnerConfigInput): ServerOutput<this> {
		const config = RunnerConfigSchema.parse(inputConfig);

		// Validate autoConfigureServerless is only used with serverless runner
		if (
			config.autoConfigureServerless &&
			config.runnerKind !== "serverless"
		) {
			throw new Error(
				"autoConfigureServerless can only be configured when runnerKind is 'serverless'",
			);
		}

		// Promise for any async operations we need to wait to complete
		const readyPromises: Promise<unknown>[] = [];

		// Disable health check if using serverless
		//
		// This is because the endpoint will not be configured until we receive
		// a serverless runner request, so we do not know what to health check
		if (config.runnerKind === "serverless") {
			logger().debug("disabling health check since using serverless");
			config.disableMetadataLookup = true;
		}

		// Auto-configure serverless runner if not in prod
		if (
			process.env.NODE_ENV !== "production" &&
			config.runnerKind === "serverless"
		) {
			if (inputConfig?.runEngine === undefined) config.runEngine = true;
			if (inputConfig?.autoConfigureServerless === undefined)
				config.autoConfigureServerless = true;
		}

		// Start engine
		if (config.runEngine) {
			logger().debug({
				msg: "run engine requested",
				version: config.runEngineVersion,
			});

			// Set config to point to the engine
			invariant(
				config.endpoint === undefined,
				"cannot specify 'endpoint' with 'runEngine'",
			);
			config.endpoint = ENGINE_ENDPOINT;
			config.disableActorDriver = true;

			// Start the engine
			const engineProcessPromise = ensureEngineProcess({
				version: config.runEngineVersion,
			});

			// Chain ready promise
			readyPromises.push(engineProcessPromise);
		}

		// Configure for serverless
		if (config.runnerKind === "serverless") {
			config.defaultServerPort = 8080;
			config.overrideServerAddress = config.endpoint;
			config.disableActorDriver = true;
			// Disable inspector for serverless runners, as they don't run actors directly
			// users can still use cloud-based inspection if needed
			config.inspector.enabled = false;
		}

		// Configure logger
		if (config.logging?.baseLogger) {
			// Use provided base logger
			configureBaseLogger(config.logging.baseLogger);
		} else {
			// Configure default logger with log level from config
			// getPinoLevel will handle env variable priority
			configureDefaultLogger(config.logging?.level);
		}

		// Choose the driver based on configuration
		const driver = chooseDefaultDriver(config);

		// Set defaults based on the driver
		if (driver.name === "engine") {
			// We need to leave the default server enabled for dev
			if (config.runnerKind !== "serverless") {
				config.disableDefaultServer = true;
			}
		}
		if (driver.name === "cloudflare-workers") {
			config.disableDefaultServer = true;
			config.disableActorDriver = true;
			config.noWelcome = true;
		}

		// Configure getUpgradeWebSocket lazily so we can assign it in crossPlatformServe
		let upgradeWebSocket: any;
		if (!config.getUpgradeWebSocket) {
			config.getUpgradeWebSocket = () => upgradeWebSocket!;
		}

		// Create router
		const managerDriver = driver.manager(this.#config, config);

		// Create client
		const client = createClientWithDriver<this>(managerDriver, config);

		const driverLog = managerDriver.extraStartupLog?.() ?? {};
		logger().info({
			msg: "rivetkit ready",
			driver: driver.name,
			definitions: Object.keys(this.#config.use).length,
			...driverLog,
		});
		if (config.inspector.enabled) {
			logger().info({
				msg: "inspector ready",
				url: getInspectorUrl(config),
			});
		}

		// Print welcome information
		if (!config.noWelcome) {
			const displayInfo = managerDriver.displayInformation();
			console.log();
			console.log(`  RivetKit ${pkg.version} (${displayInfo.name})`);
			if (!config.disableDefaultServer) {
				console.log(`  - Endpoint:     ${getEndpoint(config)}`);
			} else if (config.overrideServerAddress) {
				console.log(
					`  - Endpoint:     ${config.overrideServerAddress}`,
				);
			}
			if (config.runEngine) {
				const padding = " ".repeat(Math.max(0, 13 - "Engine".length));
				console.log(`  - Engine:${padding}v${config.runEngineVersion}`);
			}
			for (const [k, v] of Object.entries(displayInfo.properties)) {
				const padding = " ".repeat(Math.max(0, 13 - k.length));
				console.log(`  - ${k}:${padding}${v}`);
			}
			if (config.inspector.enabled) {
				console.log(`  - Inspector:    ${getInspectorUrl(config)}`);
			}
			console.log();
		}

		const { router: hono } = createManagerRouter(
			this.#config,
			config,
			managerDriver,
			driver,
			client,
		);

		// Start server
		if (!config.disableDefaultServer) {
			const serverPromise = (async () => {
				const out = await crossPlatformServe(config, hono);
				upgradeWebSocket = out.upgradeWebSocket;
			})();
			readyPromises.push(serverPromise);
		}

		// HACK: We need to find a better way to let the driver itself decide when to start the actor driver
		// Create runner
		//
		// Even though we do not use the returned ActorDriver, this is required to start the code that will handle incoming actors
		if (!config.disableActorDriver) {
			Promise.all(readyPromises).then(async () => {
				logger().debug("starting actor driver");
				driver.actor(this.#config, config, managerDriver, client);
			});
		}

		// Configure serverless runner if enabled when actor driver is disabled
		if (
			config.runnerKind === "serverless" &&
			config.autoConfigureServerless
		) {
			Promise.all(readyPromises).then(async () => {
				await configureServerlessRunner(config);
			});
		}

		return {
			client,
			fetch: hono.fetch.bind(hono),
		};
	}
}

async function configureServerlessRunner(config: RunnerConfig): Promise<void> {
	logger().debug("configuring serverless runner");

	try {
		// Ensure we have required config values
		if (!config.runnerName) {
			throw new Error(
				"runnerName is required for serverless configuration",
			);
		}
		if (!config.namespace) {
			throw new Error(
				"namespace is required for serverless configuration",
			);
		}
		if (!config.endpoint) {
			throw new Error(
				"endpoint is required for serverless configuration",
			);
		}

		// Prepare the configuration
		const customConfig =
			typeof config.autoConfigureServerless === "object"
				? config.autoConfigureServerless
				: {};

		// Create a ClientConfig from RunnerConfig for API calls
		const clientConfig: ClientConfig = {
			endpoint: config.endpoint,
			token: config.token,
			namespace: config.namespace,
			runnerName: config.runnerName,
			encoding: config.encoding,
			headers: config.headers,
			getUpgradeWebSocket: config.getUpgradeWebSocket,
			disableMetadataLookup: true, // We don't need health check for this operation
		};

		// Fetch all datacenters
		logger().debug({
			msg: "fetching datacenters",
			endpoint: config.endpoint,
		});
		const dcsRes = await getDatacenters(clientConfig);

		// Build the request body
		logger().debug({
			msg: "configuring serverless runner",
			runnerName: config.runnerName,
			namespace: config.namespace,
		});
		const serverlessConfig = {
			serverless: {
				url:
					customConfig.url ||
					`http://localhost:${config.defaultServerPort}`,
				headers: customConfig.headers || {},
				max_runners: customConfig.maxRunners ?? 100,
				min_runners: customConfig.minRunners ?? 0,
				request_lifespan: customConfig.requestLifespan ?? 15 * 60,
				runners_margin: customConfig.runnersMargin ?? 0,
				slots_per_runner:
					customConfig.slotsPerRunner ?? config.totalSlots ?? 1000,
			},
			metadata: customConfig.metadata ?? {},
		};
		await updateRunnerConfig(clientConfig, config.runnerName, {
			datacenters: Object.fromEntries(
				dcsRes.datacenters.map((dc) => [dc.name, serverlessConfig]),
			),
		});

		logger().info({
			msg: "serverless runner configured successfully",
			runnerName: config.runnerName,
			namespace: config.namespace,
		});
	} catch (error) {
		logger().error({
			msg: "failed to configure serverless runner, validate endpoint is configured correctly then restart this process",
			error,
		});

		// Don't throw, allow the runner to continue
	}
}

export function setup<A extends RegistryActors>(
	input: RegistryConfigInput<A>,
): Registry<A> {
	const config = RegistryConfigSchema.parse(input);
	return new Registry(config);
}

export type {
	RegistryConfig,
	RegistryActors,
	RunnerConfig as RunConfig,
	RunnerConfigInput as RunConfigInput,
	DriverConfig,
};
export { RegistryConfigSchema };
