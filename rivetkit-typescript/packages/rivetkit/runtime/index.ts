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
import type { Registry } from "@/registry";

/** Tracks whether the runtime was started as serverless or runner. */
export type StartKind = "serverless" | "runner";

function logLine(label: string, value: string): void {
	const padding = " ".repeat(Math.max(0, 13 - label.length));
	console.log(`  - ${label}:${padding}${value}`);
}

/**
 * Manages the lifecycle of RivetKit.
 *
 * Startup happens in two phases:
 * 1. `Runtime.create()` initializes shared infrastructure like the manager
 *    server and engine process. This runs before we know the deployment mode.
 * 2. `startServerless()` or `startRunner()` configures mode-specific behavior.
 *    These are idempotent and called lazily when the first request arrives
 *    or when explicitly starting a runner.
 */
export class Runtime<A extends RegistryActors> {
	#registry: Registry<A>;
	#config: RegistryConfig;
	#driver: DriverConfig;
	#managerDriver: ManagerDriver;
	#startKind?: StartKind;

	managerPort?: number;
	#serverlessRouter?: ReturnType<typeof buildServerlessRouter>["router"];

	get config() {
		return this.#config;
	}

	get driver() {
		return this.#driver;
	}

	get managerDriver() {
		return this.#managerDriver;
	}

	/** Use Runtime.create() instead */
	private constructor(
		registry: Registry<A>,
		config: RegistryConfig,
		driver: DriverConfig,
		managerDriver: ManagerDriver,
		managerPort?: number,
	) {
		this.#registry = registry;
		this.#config = config;
		this.#driver = driver;
		this.#managerDriver = managerDriver;
		this.managerPort = managerPort;
	}

	static async create<A extends RegistryActors>(
		registry: Registry<A>,
	): Promise<Runtime<A>> {
		logger().info("rivetkit starting");

		const config = registry.parseConfig();

		if (config.logging?.baseLogger) {
			configureBaseLogger(config.logging.baseLogger);
		} else {
			configureDefaultLogger(config.logging?.level);
		}

		// This should be unreachable: Zod defaults serveManager to false when
		// spawnEngine is enabled (since endpoint gets set to ENGINE_ENDPOINT).
		// We check anyway as a safety net for explicit misconfiguration.
		invariant(
			!(config.serverless.spawnEngine && config.serveManager),
			"cannot specify both spawnEngine and serveManager",
		);

		const driver = chooseDefaultDriver(config);
		const managerDriver = driver.manager(config);

		// Start main server. This is either:
		// - Manager: Run a server in-process on port 6420 that mimics the
		//   engine's API for development.
		// - Engine: Download and run the full Rivet engine binary on port
		//   6420. This is a fallback for platforms that cannot use the manager
		//   like Next.js.
		//
		// We do this before startServerless or startRunner has been called
		// since the engine API needs to be available on port 6420 before
		// anything else happens. For example, serverless platforms use
		// `registry.handler(req)` so `startServerless` is called lazily.
		// Starting the server preemptively allows for clients to reach 6420
		// BEFORE `startServerless` is called.
		let managerPort: number | undefined;
		if (config.serverless.spawnEngine) {
			managerPort = ENGINE_PORT;
			logger().debug({
				msg: "spawning engine",
				version: config.serverless.engineVersion,
			});
			await ensureEngineProcess({
				version: config.serverless.engineVersion,
			});
		} else if (config.serveManager) {
			const configuredManagerPort = config.managerPort;
			let upgradeWebSocket: any;
			const getUpgradeWebSocket: GetUpgradeWebSocket = () =>
				upgradeWebSocket;
			managerDriver.setGetUpgradeWebSocket(getUpgradeWebSocket);

			const { router: managerRouter } = buildManagerRouter(
				config,
				managerDriver,
				getUpgradeWebSocket,
			);

			managerPort = await findFreePort(config.managerPort);

			logger().debug({
				msg: "serving manager",
				port: managerPort,
			});

			// `publicEndpoint` is derived from `config.managerPort` during config parsing,
			// but we may have chosen a different free port at runtime. Keep them in sync
			// so browser clients that rely on `/metadata` connect to the correct manager.
			//
			// Only rewrite when `publicEndpoint` is still on the default localhost pattern,
			// to avoid clobbering explicitly-configured public endpoints.
			if (
				config.publicEndpoint ===
				`http://127.0.0.1:${configuredManagerPort}`
			) {
				config.publicEndpoint = `http://127.0.0.1:${managerPort}`;
				config.serverless.publicEndpoint = config.publicEndpoint;
			}
			config.managerPort = managerPort;

			const out = await crossPlatformServe(
				config,
				managerPort,
				managerRouter,
			);
			upgradeWebSocket = out.upgradeWebSocket;
		}

		// Create runtime
		const runtime = new Runtime(
			registry,
			config,
			driver,
			managerDriver,
			managerPort,
		);

		// Log ready
		const driverLog = managerDriver.extraStartupLog?.() ?? {};
		logger().info({
			msg: "rivetkit ready",
			driver: driver.name,
			definitions: Object.keys(config.use).length,
			...driverLog,
		});

		return runtime;
	}

	startServerless(): void {
		if (this.#startKind === "serverless") return;
		invariant(!this.#startKind, "Runtime already started as runner");
		this.#startKind = "serverless";

		this.#serverlessRouter = buildServerlessRouter(
			this.#driver,
			this.#config,
		).router;

		this.#printWelcome();

		if (this.#config.serverless.configureRunnerPool) {
			// biome-ignore lint/nursery/noFloatingPromises: intentional
			configureServerlessRunner(this.#config);
		}
	}

	startRunner(): void {
		if (this.#startKind === "runner") return;
		invariant(!this.#startKind, "Runtime already started as serverless");
		this.#startKind = "runner";

		if (this.#config.runner && this.#driver.autoStartActorDriver) {
			logger().debug("starting actor driver");
			const inlineClient = createClientWithDriver<Registry<A>>(
				this.#managerDriver,
			);
			this.#driver.actor(this.#config, this.#managerDriver, inlineClient);
		}

		this.#printWelcome();
	}

	#printWelcome(): void {
		if (this.#config.noWelcome) return;

		const inspectorUrl = this.managerPort
			? getInspectorUrl(this.#config, this.managerPort)
			: undefined;

		console.log();
		console.log(
			`  RivetKit ${pkg.version} (${this.#driver.displayName} - ${this.#startKind === "serverless" ? "Serverless" : "Runner"})`,
		);

		// Show namespace
		if (this.#config.namespace !== "default") {
			logLine("Namespace", this.#config.namespace);
		}

		// Show backend endpoint (where we connect to engine)
		if (this.#config.endpoint) {
			const endpointType = this.#config.serverless.spawnEngine
				? "local native"
				: this.#config.serveManager
					? "local manager"
					: "remote";
			logLine("Endpoint", `${this.#config.endpoint} (${endpointType})`);
		}

		// Show public endpoint (where clients connect)
		if (this.#startKind === "serverless" && this.#config.publicEndpoint) {
			logLine("Client", this.#config.publicEndpoint);
		}

		// Show inspector
		if (inspectorUrl && this.#config.inspector.enabled) {
			logLine("Inspector", inspectorUrl);
		}

		// Show actor count
		const actorCount = Object.keys(this.#config.use).length;
		logLine("Actors", actorCount.toString());

		// Show driver-specific info
		const displayInfo = this.#managerDriver.displayInformation();
		for (const [k, v] of Object.entries(displayInfo.properties)) {
			logLine(k, v);
		}

		console.log();
	}

	/** Handle serverless request */
	handleServerlessRequest(request: Request): Response | Promise<Response> {
		invariant(
			this.#startKind === "serverless",
			"not started as serverless",
		);
		invariant(this.#serverlessRouter, "serverless router not initialized");
		return this.#serverlessRouter.fetch(request);
	}
}
