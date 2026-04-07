import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { createClientWithDriver } from "@/client/client";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import {
	ENGINE_ENDPOINT,
	ENGINE_PORT,
	ensureEngineProcess,
} from "@/engine-process/mod";
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

	httpPort?: number;
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
		httpPort?: number,
	) {
		this.#registry = registry;
		this.#config = config;
		this.#engineClient = engineClient;
		this.httpPort = httpPort;
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

		if (config.startEngine) {
			config.endpoint = ENGINE_ENDPOINT;

			logger().debug({
				msg: "spawning engine",
				version: config.engineVersion,
			});
			await ensureEngineProcess({
				version: config.engineVersion,
			});
		}

		const engineClient: EngineControlClient = new RemoteEngineControlClient(
			convertRegistryConfigToClientConfig(config),
		);
		await ensureLocalRunnerConfig(config);

		const runtime = new Runtime(registry, config, engineClient);

		logger().info({
			msg: "rivetkit ready",
			driver: "engine",
			definitions: Object.keys(config.use).length,
			...(engineClient.extraStartupLog?.() ?? {}),
		});

		return runtime;
	}

	async ensureHttpServer(): Promise<void> {
		if (this.httpPort) {
			return;
		}

		const configuredHttpPort = this.#config.httpPort;
		const serveRuntime = detectRuntime();
		let upgradeWebSocket: any;
		const getUpgradeWebSocket: GetUpgradeWebSocket = () => upgradeWebSocket;
		this.#engineClient.setGetUpgradeWebSocket(getUpgradeWebSocket);

		const { router: runtimeRouter } = buildRuntimeRouter(
			this.#config,
			this.#engineClient,
			getUpgradeWebSocket,
			serveRuntime,
		);

		const httpPort = await findFreePort(configuredHttpPort);
		if (httpPort !== configuredHttpPort) {
			logger().warn({
				msg: `port ${configuredHttpPort} is in use, using ${httpPort}`,
			});
		}

		logger().debug({
			msg: "serving local HTTP server",
			port: httpPort,
		});

		if (
			this.#config.publicEndpoint ===
			`http://127.0.0.1:${configuredHttpPort}`
		) {
			this.#config.publicEndpoint = `http://127.0.0.1:${httpPort}`;
			this.#config.serverless.publicEndpoint =
				this.#config.publicEndpoint;
		}
		this.#config.httpPort = httpPort;

		let serverApp = runtimeRouter;
		if (this.#config.staticDir) {
			let dirExists = false;
			try {
				const fsSync = getNodeFsSync();
				dirExists = fsSync.existsSync(this.#config.staticDir);
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
					serveStaticFn({ root: `./${this.#config.staticDir}` }),
				);
				wrapper.route("/", runtimeRouter);
				serverApp = wrapper;
			}
		}

		const out = await crossPlatformServe(
			this.#config,
			httpPort,
			serverApp,
			serveRuntime,
		);
		upgradeWebSocket = out.upgradeWebSocket;

		if (out.closeServer && process.env.NODE_ENV !== "production") {
			const shutdown = () => {
				out.closeServer!();
			};
			process.on("SIGTERM", shutdown);
			process.on("SIGINT", shutdown);
		}

		this.httpPort = httpPort;
	}

	startServerless(): void {
		if (this.#startKind === "serverless") return;
		invariant(!this.#startKind, "Runtime already started as serverful");
		this.#startKind = "serverless";

		this.#serverlessRouter = buildServerlessRouter(this.#config).router;

		this.#printWelcome();

		if (this.#config.configurePool) {
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
			await this.#actorDriver.waitForReady();
		}

		this.#printWelcome();
	}

	#printWelcome(): void {
		if (this.#config.noWelcome) return;

		const inspectorUrl = this.httpPort
			? getInspectorUrl(this.#config, this.httpPort)
			: undefined;

		console.log();
		console.log(
			`  RivetKit ${pkg.version} (Engine - ${this.#startKind === "serverless" ? "Serverless" : "Serverful"})`,
		);

		if (this.#config.namespace !== "default") {
			logLine("Namespace", this.#config.namespace);
		}

		if (this.#config.endpoint) {
			const endpointType =
				this.#config.endpoint === ENGINE_ENDPOINT
					? "local native"
					: "remote";
			logLine("Endpoint", `${this.#config.endpoint} (${endpointType})`);
		}

		if (this.#startKind === "serverless" && this.#config.publicEndpoint) {
			logLine("Client", this.#config.publicEndpoint);
		}

		if (this.#config.staticDir) {
			try {
				const fsSync = getNodeFsSync();
				if (fsSync.existsSync(this.#config.staticDir)) {
					logLine("Static", `./${this.#config.staticDir}`);
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
