import invariant from "invariant";
import { serve as honoServe } from "@hono/node-server";
import { Hono } from "hono";
import { type TestContext } from "vitest";
import { createClientWithDriver } from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { type Client, createClient } from "@/client/mod";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { ENGINE_ENDPOINT, ensureEngineProcess } from "@/engine-process/mod";
import { updateRunnerConfig } from "@/engine-client/api-endpoints";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import { EngineActorDriver } from "@/drivers/engine/mod";
import { type Registry } from "@/mod";

export interface SetupTestResult<A extends Registry<any>> {
	client: Client<A>;
}

async function ensureNamespaceExists(
	endpoint: string,
	namespace: string,
	token: string,
): Promise<void> {
	const response = await fetch(`${endpoint}/namespaces`, {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({
			name: namespace,
			display_name: namespace,
		}),
	});

	if (response.ok || response.status === 409) {
		return;
	}

	throw new Error(
		`create namespace failed: ${response.status} ${await response.text()}`,
	);
}

async function closeNodeServer(
	server: ReturnType<typeof honoServe>,
): Promise<void> {
	await new Promise<void>((resolve, reject) => {
		server.close((error) => {
			if (error) {
				reject(error);
				return;
			}
			resolve();
		});

		server.closeIdleConnections?.();
		server.closeAllConnections?.();
	});
}

async function refreshRunnerMetadata(
	endpoint: string,
	namespace: string,
	token: string,
	poolName: string,
): Promise<void> {
	let lastError: unknown;

	for (let attempt = 0; attempt < 20; attempt += 1) {
		try {
			const response = await fetch(
				`${endpoint}/runner-configs/${encodeURIComponent(poolName)}/refresh-metadata?namespace=${encodeURIComponent(namespace)}`,
				{
					method: "POST",
					headers: {
						Authorization: `Bearer ${token}`,
						"Content-Type": "application/json",
					},
					body: JSON.stringify({}),
					signal: AbortSignal.timeout(2_000),
				},
			);
			if (response.ok) {
				return;
			}
			lastError = new Error(
				`refresh runner metadata failed: ${response.status} ${await response.text()}`,
			);
		} catch (error) {
			lastError = error;
		}

		if (attempt < 19) {
			await new Promise((resolve) => setTimeout(resolve, 250));
		}
	}

	throw lastError;
}

// Must use `TestContext` since global hooks do not work when running concurrently
export async function setupTest<A extends Registry<any>>(
	c: TestContext,
	registry: A,
): Promise<SetupTestResult<A>> {
	const testId = crypto.randomUUID();

	registry.config.test = { ...registry.config.test, enabled: true };
	registry.config.namespace ??= `test-${testId}`;
	registry.config.inspector = {
		enabled: true,
		token: () => "token",
	};
	registry.config.envoy = {
		...registry.config.envoy,
		poolName: registry.config.envoy?.poolName ?? `test-${testId}`,
	};

	const parsedConfig = registry.parseConfig();
	const shouldSpawnEngine =
		parsedConfig.serverless.spawnEngine ||
		!parsedConfig.endpoint;
	if (shouldSpawnEngine) {
		await ensureEngineProcess({
			version: parsedConfig.serverless.engineVersion,
		});
	}

	const endpoint =
		parsedConfig.endpoint ?? (shouldSpawnEngine ? ENGINE_ENDPOINT : undefined);
	const token =
		parsedConfig.token ??
		(endpoint === ENGINE_ENDPOINT ? "dev" : undefined);
	if (endpoint && !registry.config.endpoint) {
		registry.config.endpoint = endpoint;
	}
	if (endpoint === ENGINE_ENDPOINT && !registry.config.token) {
		registry.config.token = token;
	}
	if (endpoint && token) {
		await ensureNamespaceExists(endpoint, parsedConfig.namespace, token);
	}

	const runtimeConfig = registry.parseConfig();
	const clientConfig = convertRegistryConfigToClientConfig(runtimeConfig);
	const engineClient = new RemoteEngineControlClient(clientConfig);
	const inlineClient = createClientWithDriver(engineClient, clientConfig);
	const actorDriver = new EngineActorDriver(
		runtimeConfig,
		engineClient,
		inlineClient,
	);

	const app = new Hono();
	app.get("/health", (ctx) => handleHealthRequest(ctx));
	app.get("/metadata", (ctx) =>
		handleMetadataRequest(
			ctx,
			runtimeConfig,
			{ serverless: {} },
			runtimeConfig.publicEndpoint,
			runtimeConfig.publicNamespace,
			runtimeConfig.publicToken,
		),
	);
	app.post("/start", async (ctx) => {
		return await actorDriver.serverlessHandleStart!(ctx);
	});

	const server = honoServe({
		fetch: app.fetch,
		hostname: "127.0.0.1",
		port: 0,
	});
	if (!server.listening) {
		await new Promise<void>((resolve) => {
			server.once("listening", () => resolve());
		});
	}
	const address = server.address();
	invariant(address && typeof address !== "string", "missing server address");
	const serverlessUrl = `http://127.0.0.1:${address.port}`;

	await updateRunnerConfig(clientConfig, runtimeConfig.envoy.poolName, {
		datacenters: {
			default: {
				serverless: {
					url: serverlessUrl,
					headers: {},
					request_lifespan: 300,
					slots_per_runner: 1,
					min_runners: 0,
					max_runners: 10000,
					runners_margin: 0,
				},
			},
		},
	});

	await actorDriver.waitForReady();
	if (endpoint && token) {
		await refreshRunnerMetadata(
			endpoint,
			runtimeConfig.namespace,
			token,
			runtimeConfig.envoy.poolName,
		);
	}

	const client = createClient<A>({
		endpoint: runtimeConfig.endpoint,
		namespace: runtimeConfig.namespace,
		poolName: runtimeConfig.envoy.poolName,
		disableMetadataLookup: true,
	});

	c.onTestFinished(async () => {
		await client.dispose();
		await actorDriver.shutdown(true);
		await closeNodeServer(server);
	});

	return { client };
}
