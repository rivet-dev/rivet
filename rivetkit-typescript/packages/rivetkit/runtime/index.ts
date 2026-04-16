import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { createClientWithDriver } from "@/client/client";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import { ENGINE_ENDPOINT, ENGINE_PORT, ensureEngineProcess } from "@/engine-process/mod";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import { type EngineControlClient } from "@/engine-client/driver";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { getInspectorUrl } from "@/inspector/utils";
import { type RegistryActors, type RegistryConfig } from "@/registry/config";
import { logger } from "../src/registry/log";
import { buildRuntimeRouter } from "@/runtime-router/router";
import { EngineActorDriver } from "@/drivers/engine/mod";
import { buildServerlessRouter } from "@/serverless/router";
import { configureServerlessPool } from "@/serverless/configure";
import { detectRuntime, type GetUpgradeWebSocket } from "@/utils";
import {
	crossPlatformServe,
	findFreePort,
	loadRuntimeServeStatic,
} from "@/utils/serve";
import type { Registry } from "@/registry";
import { getNodeFsSync } from "@/utils/node";
import pkg from "../package.json" with { type: "json" };

/** Tracks whether the runtime was started as serverless or serverful. */
export type StartKind = "serverless" | "serverful";

function logLine(label: string, value: string): void {
	const padding = " ".repeat(Math.max(0, 13 - label.length));
	console.log(`  - ${label}:${padding}${value}`);
}

async function ensureLocalRunnerConfig(config: RegistryConfig): Promise<void> {
	if (config.endpoint !== ENGINE_ENDPOINT) {
		return;
	}

	const clientConfig = convertRegistryConfigToClientConfig(config);
	const dcsRes = await getDatacenters(clientConfig);

	await updateRunnerConfig(clientConfig, config.envoy.poolName, {
		datacenters: Object.fromEntries(
			dcsRes.datacenters.map((dc) => [
				dc.name,
				{
					normal: {},
					drain_on_version_upgrade: true,
				},
			]),
		),
	});
}

export class Runtime<A extends RegistryActors> {
	#registry: Registry<A>;
	#config: RegistryConfig;
	#engineClient: EngineControlClient;
	#actorDriver?: EngineActorDriver;
	#startKind?: StartKind;

	managerPort?: number;
	#serverlessRouter?: ReturnType<typeof buildServerlessRouter>["router"];

	get config() {
		return this.#config;
	}

	get engineClient() {
		return this.#engineClient;
	}

	private constructor(
		registry: Registry<A>,
		config: RegistryConfig,
		engineClient: EngineControlClient,
		managerPort?: number,
	) {
		this.#registry = registry;
		this.#config = config;
		this.#engineClient = engineClient;
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

		const shouldSpawnEngine =
			config.serverless.spawnEngine || (config.serveManager && !config.endpoint);
		let engineChildProcess: import("node:child_process").ChildProcess | undefined;
		if (shouldSpawnEngine) {
			config.endpoint = ENGINE_ENDPOINT;

			logger().debug({
				msg: "spawning engine",
				version: config.serverless.engineVersion,
			});
			engineChildProcess = await ensureEngineProcess({
				version: config.serverless.engineVersion,
				managerPort: config.managerPort,
			});
		}

		const engineClient: EngineControlClient = new RemoteEngineControlClient(
			convertRegistryConfigToClientConfig(config),
		);
		await ensureLocalRunnerConfig(config);

		let managerPort: number | undefined;
		if (config.serveManager) {
			const configuredManagerPort = config.managerPort;
			const serveRuntime = detectRuntime();
			let upgradeWebSocket: any;
			const getUpgradeWebSocket: GetUpgradeWebSocket = () =>
				upgradeWebSocket;
			engineClient.setGetUpgradeWebSocket(getUpgradeWebSocket);

			const { router: runtimeRouter } = buildRuntimeRouter(
				config,
				engineClient,
				getUpgradeWebSocket,
				serveRuntime,
			);

			managerPort = await findFreePort(config.managerPort);

			if (managerPort !== configuredManagerPort) {
				logger().warn({
					msg: `port ${configuredManagerPort} is in use, using ${managerPort}`,
				});
			}

			logger().debug({
				msg: "serving runtime router",
				port: managerPort,
			});

			if (
				config.publicEndpoint ===
				`http://127.0.0.1:${configuredManagerPort}`
			) {
				config.publicEndpoint = `http://127.0.0.1:${managerPort}`;
				config.serverless.publicEndpoint = config.publicEndpoint;
			}
			config.managerPort = managerPort;

			let serverApp = runtimeRouter;
			if (config.publicDir) {
				let dirExists = false;
				try {
					const fsSync = getNodeFsSync();
					dirExists = fsSync.existsSync(config.publicDir);
				} catch {
					// Node fs not available.
				}

				if (dirExists) {
					const { Hono } = await import("hono");
					const serveStaticFn =
						await loadRuntimeServeStatic(serveRuntime);
					const wrapper = new Hono();
					wrapper.use(
						"*",
						serveStaticFn({ root: `./${config.publicDir}` }),
					);
					wrapper.route("/", runtimeRouter);
					serverApp = wrapper;
				}
			}

			const out = await crossPlatformServe(
				config,
				managerPort,
				serverApp,
				serveRuntime,
			);
			upgradeWebSocket = out.upgradeWebSocket;

			if (out.closeServer && process.env.NODE_ENV !== "production") {
				const shutdown = () => {
					out.closeServer!();
					engineChildProcess?.kill("SIGTERM");
				};
				process.on("SIGTERM", shutdown);
				process.on("SIGINT", shutdown);
			}
		} else if (engineChildProcess && process.env.NODE_ENV !== "production") {
			const shutdown = () => {
				engineChildProcess.kill("SIGTERM");
			};
			process.on("SIGTERM", shutdown);
			process.on("SIGINT", shutdown);
		}

		const runtime = new Runtime(registry, config, engineClient, managerPort);

		logger().info({
			msg: "rivetkit ready",
			driver: "engine",
			definitions: Object.keys(config.use).length,
			...(engineClient.extraStartupLog?.() ?? {}),
		});

		return runtime;
	}

	startServerless(): void {
		if (this.#startKind === "serverless") return;
		invariant(!this.#startKind, "Runtime already started as serverful");
		this.#startKind = "serverless";

		this.#serverlessRouter = buildServerlessRouter(this.#config).router;

		this.#printWelcome();

		if (this.#config.serverless.configurePool) {
			// biome-ignore lint/nursery/noFloatingPromises: intentional
			configureServerlessPool(this.#config);
		}
	}

	async startEnvoy(): Promise<void> {
		if (this.#startKind === "serverful") return;
		invariant(!this.#startKind, "Runtime already started as serverless");
		this.#startKind = "serverful";

		if (this.#config.envoy && !this.#actorDriver) {
			logger().debug("starting engine actor driver");
			const inlineClient = createClientWithDriver<Registry<A>>(
				this.#engineClient,
			);
			this.#actorDriver = new EngineActorDriver(
				this.#config,
				this.#engineClient,
				inlineClient,
			);
			logger().info({ msg: "connecting to engine" });
			try {
				await this.#actorDriver.waitForReady();
				logger().info({ msg: "connected to engine" });
			} catch (err) {
				logger().error({ msg: "failed to connect to engine", err });
				throw err;
			}
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
			`  RivetKit ${pkg.version} (Engine - ${this.#startKind === "serverless" ? "Serverless" : "Serverful"})`,
		);

		if (this.#config.namespace !== "default") {
			logLine("Namespace", this.#config.namespace);
		}

		if (this.#config.endpoint) {
			const endpointType = this.#config.endpoint === ENGINE_ENDPOINT
				? "local native"
				: "remote";
			logLine("Endpoint", `${this.#config.endpoint} (${endpointType})`);
		}

		if (this.#startKind === "serverless" && this.#config.publicEndpoint) {
			logLine("Client", this.#config.publicEndpoint);
		}

		if (this.#config.publicDir) {
			try {
				const fsSync = getNodeFsSync();
				if (fsSync.existsSync(this.#config.publicDir)) {
					logLine("Static", `./${this.#config.publicDir}`);
				}
			} catch {
				// Node fs not available.
			}
		}

		if (inspectorUrl && this.#config.inspector.enabled) {
			logLine("Inspector", inspectorUrl);
		}

		logLine("Actors", Object.keys(this.#config.use).length.toString());

		const displayInfo = this.#engineClient.displayInformation();
		for (const [k, v] of Object.entries(displayInfo.properties)) {
			logLine(k, v);
		}

		console.log();
	}

	handleServerlessRequest(request: Request): Response | Promise<Response> {
		invariant(
			this.#startKind === "serverless",
			"not started as serverless",
		);
		invariant(this.#serverlessRouter, "serverless router not initialized");
		return this.#serverlessRouter.fetch(request);
	}
}
