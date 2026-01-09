import invariant from "invariant";
import { createClientWithDriver } from "@/client/client";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import { chooseDefaultDriver } from "@/drivers/default";
import { ENGINE_PORT, ensureEngineProcess } from "@/engine-process/mod";
import { getInspectorUrl } from "@/inspector/utils";
import { buildManagerRouter } from "@/manager/router";
import { configureServerlessRunner } from "@/serverless/configure";
import type { GetUpgradeWebSocket } from "@/utils";
import pkg from "../package.json" with { type: "json" };
import {
	type DriverConfig,
	type RegistryActors,
	type RegistryConfig,
} from "@/registry/config";
import { logger } from "../src/registry/log";
import { crossPlatformServe, findFreePort } from "@/registry/serve";
import { ManagerDriver } from "@/manager/driver";
import { buildServerlessRouter } from "@/serverless/router";
import { Registry } from "@/registry";

/**
 * Defines what type of server is being started. Used internally for
 * Registry.#start
 **/
export type StartKind = "serverless" | "runner";

export class Runtime<A extends RegistryActors> {
	#registry: Registry<A>;
	managerPort?: number;
	#config: RegistryConfig;
	#driver: DriverConfig;
	#kind: StartKind;
	#managerDriver: ManagerDriver;

	get config() {
		return this.#config;
	}

	get driver() {
		return this.#driver;
	}

	get managerDriver() {
		return this.#managerDriver;
	}

	#serverlessRouter?: ReturnType<typeof buildServerlessRouter>["router"];

	constructor(registry: Registry<A>, kind: StartKind) {
		this.#registry = registry;
		this.#kind = kind;

		const config = this.#registry.parseConfig();
		this.#config = config;

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
			this.managerPort = ENGINE_PORT;

			logger().debug({
				msg: "run engine requested",
				version: config.serverless.engineVersion,
			});

			// Start the engine
			const engineProcessPromise = ensureEngineProcess({
				version: config.serverless.engineVersion,
			});

			// Chain ready promise
			readyPromises.push(engineProcessPromise);
		}

		// Choose the driver based on configuration
		const driver = chooseDefaultDriver(config);

		// Create manager driver (always needed for actor driver + inline client)
		const managerDriver = driver.manager(config);

		// Start manager
		if (config.serveManager) {
			// Configure getUpgradeWebSocket lazily so we can assign it in crossPlatformServe
			let upgradeWebSocket: any;
			const getUpgradeWebSocket: GetUpgradeWebSocket = () =>
				upgradeWebSocket;
			managerDriver.setGetUpgradeWebSocket(getUpgradeWebSocket);

			// Build router
			const { router: managerRouter } = buildManagerRouter(
				config,
				managerDriver,
				getUpgradeWebSocket,
			);

			// Serve manager
			const serverPromise = (async () => {
				const managerPort = await findFreePort(config.managerPort);
				this.managerPort = managerPort;

				const out = await crossPlatformServe(
					config,
					managerPort,
					managerRouter,
				);
				upgradeWebSocket = out.upgradeWebSocket;
			})();
			readyPromises.push(serverPromise);
		}

		// Build serverless router
		if (kind === "serverless") {
			this.#serverlessRouter = buildServerlessRouter(
				driver,
				config,
			).router;
		}

		this.#driver = driver;
		this.#managerDriver = managerDriver;

		// Log and print welcome after all ready promises complete
		// biome-ignore lint/nursery/noFloatingPromises: bg promise
		Promise.all(readyPromises).then(async () => this.#onAfterReady());
	}

	async #onAfterReady() {
		const config = this.#config;
		const kind = this.#kind;
		const driver = this.#driver;
		const managerDriver = this.#managerDriver;

		// Auto-start actor driver for drivers that require it.
		//
		// This is only enabled for runner config since serverless will
		// auto-start the actor driver on `GET /start`.
		if (kind === "runner" && config.runner && driver.autoStartActorDriver) {
			logger().debug("starting actor driver");
			const inlineClient =
				createClientWithDriver<Registry<A>>(managerDriver);
			driver.actor(config, managerDriver, inlineClient);
		}

		// Log starting
		const driverLog = managerDriver.extraStartupLog?.() ?? {};
		logger().info({
			msg: "rivetkit ready",
			driver: driver.name,
			definitions: Object.keys(config.use).length,
			...driverLog,
		});
		invariant(this.managerPort, "managerPort should be set");
		const inspectorUrl = getInspectorUrl(config, this.managerPort);
		if (inspectorUrl && config.inspector.enabled) {
			logger().info({
				msg: "inspector ready",
				url: inspectorUrl,
			});
		}

		// Print welcome information
		if (!config.noWelcome) {
			console.log();
			console.log(`  RivetKit ${pkg.version} (${driver.displayName})`);
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
				const padding = " ".repeat(Math.max(0, 13 - "Engine".length));
				console.log(
					`  - Engine:${padding}v${config.serverless.engineVersion}`,
				);
			}
			const displayInfo = managerDriver.displayInformation();
			for (const [k, v] of Object.entries(displayInfo.properties)) {
				const padding = " ".repeat(Math.max(0, 13 - k.length));
				console.log(`  - ${k}:${padding}${v}`);
			}
			if (inspectorUrl && config.inspector.enabled) {
				console.log(`  - Inspector:    ${inspectorUrl}`);
			}
			console.log();
		}

		// Configure serverless runner if enabled when actor driver is disabled
		if (kind === "serverless" && config.serverless.configureRunnerPool) {
			await configureServerlessRunner(config);
		}
	}

	public handler(request: Request): Response | Promise<Response> {
		invariant(this.#kind === "serverless", "kind not serverless");
		invariant(this.#serverlessRouter, "missing serverless router");
		return this.#serverlessRouter.fetch(request);
	}
}
