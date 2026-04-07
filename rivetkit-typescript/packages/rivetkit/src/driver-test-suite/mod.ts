import { serve as honoServe } from "@hono/node-server";
import { createNodeWebSocket } from "@hono/node-ws";
import invariant from "invariant";
import { describe } from "vitest";
import type { Encoding } from "@/client/mod";
import { buildRuntimeRouter } from "@/runtime-router/router";
import { type Registry } from "@/mod";
import type { EngineControlClient } from "@/engine-client/driver";
import { logger } from "./log";
import { runActionFeaturesTests } from "./tests/action-features";
import { runAccessControlTests } from "./tests/access-control";
import { runActorConnTests } from "./tests/actor-conn";
import { runActorConnHibernationTests } from "./tests/actor-conn-hibernation";
import { runActorConnStateTests } from "./tests/actor-conn-state";
import { runActorDbTests } from "./tests/actor-db";
import { runActorDbStressTests } from "./tests/actor-db-stress";
import { runConnErrorSerializationTests } from "./tests/conn-error-serialization";
import { runActorDestroyTests } from "./tests/actor-destroy";
import { runActorDriverTests } from "./tests/actor-driver";
import { runActorErrorHandlingTests } from "./tests/actor-error-handling";
import { runActorHandleTests } from "./tests/actor-handle";
import { runActorInlineClientTests } from "./tests/actor-inline-client";
import { runActorInspectorTests } from "./tests/actor-inspector";
import { runActorKvTests } from "./tests/actor-kv";
import { runActorMetadataTests } from "./tests/actor-metadata";
import { runActorOnStateChangeTests } from "./tests/actor-onstatechange";
import { runActorQueueTests } from "./tests/actor-queue";
import { runDynamicReloadTests } from "./tests/dynamic-reload";
import { runActorRunTests } from "./tests/actor-run";
import { runActorSandboxTests } from "./tests/actor-sandbox";
import { runActorStatelessTests } from "./tests/actor-stateless";
import { runActorVarsTests } from "./tests/actor-vars";
import { runActorWorkflowTests } from "./tests/actor-workflow";
import { runCrossBackendVfsTests } from "./tests/cross-backend-vfs";
import { runManagerDriverTests } from "./tests/manager-driver";
import { runRawHttpTests } from "./tests/raw-http";
import { runRawHttpRequestPropertiesTests } from "./tests/raw-http-request-properties";
import { runRawWebSocketTests } from "./tests/raw-websocket";
import { runActorDbKvStatsTests } from "./tests/actor-db-kv-stats";
import { runActorDbPragmaMigrationTests } from "./tests/actor-db-pragma-migration";
import { runActorStateZodCoercionTests } from "./tests/actor-state-zod-coercion";
import { runActorAgentOsTests } from "./tests/actor-agent-os";
import { runGatewayQueryUrlTests } from "./tests/gateway-query-url";
import { runHibernatableWebSocketProtocolTests } from "./tests/hibernatable-websocket-protocol";
import { runRequestAccessTests } from "./tests/request-access";

export interface SkipTests {
	schedule?: boolean;
	sleep?: boolean;
	hibernation?: boolean;
	inline?: boolean;
	sandbox?: boolean;
	agentOs?: boolean;
}

export interface DriverTestFeatures {
	hibernatableWebSocketProtocol?: boolean;
}

export interface DriverTestConfig {
	/** Deploys an registry and returns the connection endpoint. */
	start(): Promise<DriverDeployOutput>;

	/**
	 * If we're testing with an external system, we should use real timers
	 * instead of Vitest's mocked timers.
	 **/
	useRealTimers?: boolean;

	/** Cloudflare Workers has some bugs with cleanup. */
	HACK_skipCleanupNet?: boolean;

	skip?: SkipTests;

	features?: DriverTestFeatures;

	/** Restrict which encodings to test. Defaults to all (bare, cbor, json). */
	encodings?: Encoding[];

	/** Restrict which client types to test. Defaults to http + inline (unless skip.inline is set). */
	clientTypes?: ClientType[];

	encoding?: Encoding;

	isDynamic?: boolean;

	clientType: ClientType;

	cleanup?: () => Promise<void>;
}

/**
 * The type of client to run the test with.
 *
 * The logic for HTTP vs inline is very different, so this helps validate all behavior matches.
 **/
type ClientType = "http" | "inline";

export interface DriverDeployOutput {
	endpoint: string;
	namespace: string;
	runnerName: string;
	hardCrashActor?: (actorId: string) => Promise<void>;
	hardCrashPreservesData?: boolean;

	/** Cleans up the test. */
	cleanup(): Promise<void>;
}

/** Runs all Vitest tests against the provided drivers. */
export function runDriverTests(
	driverTestConfigPartial: Omit<DriverTestConfig, "clientType" | "encoding">,
) {
	describe("Driver Tests", () => {
		const clientTypes: ClientType[] = driverTestConfigPartial.clientTypes
			?? (driverTestConfigPartial.skip?.inline ? ["http"] : ["http", "inline"]);
		for (const clientType of clientTypes) {
			describe(`client type (${clientType})`, () => {
				const encodings: Encoding[] = driverTestConfigPartial.encodings ?? ["bare", "cbor", "json"];

				for (const encoding of encodings) {
					describe(`encoding (${encoding})`, () => {
						const driverTestConfig: DriverTestConfig = {
							...driverTestConfigPartial,
							clientType,
							encoding,
						};

						runActorDriverTests(driverTestConfig);
						runManagerDriverTests(driverTestConfig);

						runActorConnTests(driverTestConfig);

						runActorConnStateTests(driverTestConfig);

						runActorConnHibernationTests(driverTestConfig);

						runConnErrorSerializationTests(driverTestConfig);

						runActorDbTests(driverTestConfig);

						runActorDestroyTests(driverTestConfig);

						runRequestAccessTests(driverTestConfig);

						runActorHandleTests(driverTestConfig);

						runActionFeaturesTests(driverTestConfig);

						runAccessControlTests(driverTestConfig);

						runActorVarsTests(driverTestConfig);

						runActorMetadataTests(driverTestConfig);

						runActorOnStateChangeTests(driverTestConfig);

						runActorErrorHandlingTests(driverTestConfig);

						runActorQueueTests(driverTestConfig);

						runActorRunTests(driverTestConfig);

						runActorSandboxTests(driverTestConfig);

						runDynamicReloadTests(driverTestConfig);

						runActorInlineClientTests(driverTestConfig);

						runActorKvTests(driverTestConfig);

						runActorWorkflowTests(driverTestConfig);

						runActorStatelessTests(driverTestConfig);

						runRawHttpTests(driverTestConfig);

						runRawHttpRequestPropertiesTests(driverTestConfig);

						runRawWebSocketTests(driverTestConfig);
						runHibernatableWebSocketProtocolTests(driverTestConfig);

						// TODO: re-expose this once we can have actor queries on the gateway
						// runRawHttpDirectRegistryTests(driverTestConfig);

						// TODO: re-expose this once we can have actor queries on the gateway
						// runRawWebSocketDirectRegistryTests(driverTestConfig);

						runActorInspectorTests(driverTestConfig);
						runGatewayQueryUrlTests(driverTestConfig);

						runActorDbKvStatsTests(driverTestConfig);

						runActorDbPragmaMigrationTests(driverTestConfig);

						runActorStateZodCoercionTests(driverTestConfig);

						runActorAgentOsTests(driverTestConfig);
					});
				}
			});
		}

		// Cross-backend VFS compatibility runs once, independent of
		// client type and encoding. Skips when native SQLite is unavailable.
		runCrossBackendVfsTests({
			...driverTestConfigPartial,
			clientType: "http",
			encoding: "bare",
		});

		// Stress tests for DB lifecycle races, event loop blocking, and
		// KV channel resilience. Run once, not per-encoding.
		runActorDbStressTests({
			...driverTestConfigPartial,
			clientType: "http",
			encoding: "bare",
		});
	});
}

/**
 * Helper function to adapt the drivers to the Node.js runtime for tests.
 *
 * This is helpful for drivers that run in-process as opposed to drivers that rely on external tools.
 */
export async function createTestRuntime(
	registryPath: string,
	driverFactory: (registry: Registry<any>) => Promise<{
		rivetEngine?: {
			endpoint: string;
			namespace: string;
			runnerName: string;
			token: string;
		};
		engineClient: EngineControlClient;
		hardCrashActor?: (actorId: string) => Promise<void>;
		hardCrashPreservesData?: boolean;
		cleanup?: () => Promise<void>;
	}>,
): Promise<DriverDeployOutput> {
	// Import using dynamic imports with vitest alias resolution
	//
	// Vitest is configured to resolve `import ... from "rivetkit"` to the
	// appropriate source files
	//
	// We need to preserve the `import ... from "rivetkit"` in the fixtures so
	// targets that run the server separately from the Vitest tests (such as
	// Cloudflare Workers) still function.
	const { registry } = (await import(registryPath)) as {
		registry: Registry<any>;
	};

	// TODO: Find a cleaner way of flagging an registry as test mode (ideally not in the config itself)
	// Force enable test
	registry.config.test = { ...registry.config.test, enabled: true };
	registry.config.inspector = {
		enabled: true,
		token: () => "token",
	};

	// Build drivers
	const {
		engineClient,
		cleanup: driverCleanup,
		rivetEngine,
		hardCrashActor,
		hardCrashPreservesData,
	} = await driverFactory(registry);

	if (rivetEngine) {
		// TODO: We don't need createTestRuntime fort his
		// Using external Rivet engine

		const cleanup = async () => {
			await driverCleanup?.();
		};

		return {
			endpoint: rivetEngine.endpoint,
			namespace: rivetEngine.namespace,
			runnerName: rivetEngine.runnerName,
			hardCrashActor,
			hardCrashPreservesData,
			cleanup,
		};
	} else {
		// Start server for Rivet engine

		// Build driver config
		// biome-ignore lint/style/useConst: Assigned later
		let upgradeWebSocket: any;

		// Create router
		const parsedConfig = registry.parseConfig();
		const managerDriver = engineClient;
		const { router } = buildRuntimeRouter(
			parsedConfig,
			managerDriver,
			() => upgradeWebSocket,
		);

		// Inject WebSocket
		const nodeWebSocket = createNodeWebSocket({ app: router });
		upgradeWebSocket = nodeWebSocket.upgradeWebSocket;
		managerDriver.setGetUpgradeWebSocket(() => upgradeWebSocket);

		// TODO: I think this whole function is fucked, we should probably switch to calling registry.serve() directly
		// Start server
		const server = honoServe({
			fetch: router.fetch,
			hostname: "127.0.0.1",
			port: 0,
		});
		if (!server.listening) {
			await new Promise<void>((resolve) => {
				server.once("listening", () => resolve());
			});
		}
		invariant(
			nodeWebSocket.injectWebSocket !== undefined,
			"should have injectWebSocket",
		);
		nodeWebSocket.injectWebSocket(server);
		const address = server.address();
		invariant(
			address && typeof address !== "string",
			"missing server address",
		);
		const port = address.port;
		const serverEndpoint = `http://127.0.0.1:${port}`;
		managerDriver.setNativeSqliteConfig?.({
			endpoint: serverEndpoint,
			namespace: "default",
		});

		logger().info({ msg: "test serer listening", port });

		// Cleanup
		const cleanup = async () => {
			// Disconnect only the current test runtime's native KV channel so
			// concurrent local runtimes do not shut down each other's channel.
			try {
				const { disconnectKvChannelForCurrentConfig } = await import(
					"@/db/native-sqlite"
				);
				await disconnectKvChannelForCurrentConfig({
					endpoint: serverEndpoint,
					namespace: "default",
				});
			} catch {
				// Native module may not be available.
			}

			// Stop server
			await new Promise((resolve) =>
				server.close(() => resolve(undefined)),
			);

			// Extra cleanup
			await driverCleanup?.();
		};

		return {
			endpoint: serverEndpoint,
			namespace: "default",
			runnerName: "default",
			hardCrashActor: managerDriver.hardCrashActor?.bind(managerDriver),
			hardCrashPreservesData: true,
			cleanup,
		};
	}
}
