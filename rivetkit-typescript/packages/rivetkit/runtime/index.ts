import { convertRegistryConfigToClientConfig } from "@/client/config";
import { configureBaseLogger, configureDefaultLogger } from "@/common/log";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import type { EngineControlClient } from "@/engine-client/driver";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import { getInspectorUrl } from "@/inspector/utils";
import type { Registry } from "@/registry";
import type { RegistryActors, RegistryConfig } from "@/registry/config";
import { getNodeFsSync } from "@/utils/node";
import pkg from "../package.json" with { type: "json" };
import { logger } from "../src/registry/log";

/** Tracks whether the runtime was started as serverless or serverful. */
export type StartKind = "serverless" | "serverful";

function removedLegacyRoutingError(method: string): Error {
	return new Error(
		`Runtime.${method}() relied on the removed TypeScript routing/serverless stack. Use Registry.startEnvoy() with the native rivetkit-core path instead.`,
	);
}

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
	#config: RegistryConfig;
	#engineClient: EngineControlClient;
	#startKind?: StartKind;

	httpPort?: number;

	get config() {
		return this.#config;
	}

	get engineClient() {
		return this.#engineClient;
	}

	private constructor(
		_registry: Registry<A>,
		config: RegistryConfig,
		engineClient: EngineControlClient,
		httpPort?: number,
	) {
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
		throw removedLegacyRoutingError("ensureHttpServer");
	}

	startServerless(): void {
		throw removedLegacyRoutingError("startServerless");
	}

	async startEnvoy(): Promise<void> {
		if (this.#startKind === "serverful") return;
		this.#startKind = "serverful";

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
		void request;
		throw removedLegacyRoutingError("handleServerlessRequest");
	}
}
