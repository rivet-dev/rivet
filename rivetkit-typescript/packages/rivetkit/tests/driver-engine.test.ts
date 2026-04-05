import { join } from "node:path";
import { createClientWithDriver } from "@/client/client";
import { createTestRuntime, runDriverTests } from "@/driver-test-suite/mod";
import type { DriverTestConfig } from "@/driver-test-suite/mod";
import { setupDriverTest } from "@/driver-test-suite/utils";
import { createEngineDriver } from "@/drivers/engine/mod";
import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { describe, expect, test, vi } from "vitest";

interface ActorListResponse {
	actors?: Array<{ actor_id?: string }>;
	pagination?: { cursor?: string | null };
}

interface ActorNamesResponse {
	names?: Record<string, unknown>;
	pagination?: { cursor?: string | null };
}

interface SharedEngineRuntime {
	endpoint: string;
	namespace: string;
	runnerName: string;
	token: string;
	driverConfig: ReturnType<typeof createEngineDriver>;
	actorDriver: ReturnType<ReturnType<typeof createEngineDriver>["actor"]>;
	forceDisconnectKvChannel: () => Promise<number>;
}

let sharedNamespacePromise: Promise<string> | undefined;
let sharedRunnerConfigPromise: Promise<string> | undefined;
let sharedEngineRuntimePromise: Promise<SharedEngineRuntime> | undefined;

async function ensureSharedNamespace(
	endpoint: string,
	token: string,
): Promise<string> {
	if (!sharedNamespacePromise) {
		sharedNamespacePromise = (async () => {
			const namespace = `test-driver-engine-${crypto.randomUUID().slice(0, 8)}`;
			const response = await fetch(`${endpoint}/namespaces`, {
				method: "POST",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${token}`,
				},
				body: JSON.stringify({
					name: namespace,
					display_name: namespace,
				}),
			});
			if (!response.ok) {
				const errorBody = await response.text().catch(() => "");
				throw new Error(
					`Create shared namespace failed at ${endpoint}: ${response.status} ${response.statusText} ${errorBody}`,
				);
			}

			return namespace;
		})();
	}

	return await sharedNamespacePromise;
}

async function ensureSharedRunnerConfig(
	endpoint: string,
	namespace: string,
	token: string,
): Promise<string> {
	if (!sharedRunnerConfigPromise) {
		sharedRunnerConfigPromise = (async () => {
			const runnerName = `test-runner-${crypto.randomUUID().slice(0, 8)}`;
			const response = await fetch(
				`${endpoint}/runner-configs/${runnerName}?namespace=${namespace}`,
				{
					method: "PUT",
					headers: {
						"Content-Type": "application/json",
						Authorization: `Bearer ${token}`,
					},
					body: JSON.stringify({
						datacenters: {
							default: { normal: {} },
						},
					}),
				},
			);
			if (!response.ok) {
				const errorBody = await response.text().catch(() => "");
				throw new Error(
					`Create shared runner config failed: ${response.status} ${response.statusText} ${errorBody}`,
				);
			}

			return runnerName;
		})();
	}

	return await sharedRunnerConfigPromise;
}

async function listAllActorNames(
	endpoint: string,
	namespace: string,
	token: string,
): Promise<string[]> {
	const names: string[] = [];
	let cursor: string | undefined;

	for (;;) {
		const url = new URL("/actors/names", endpoint);
		url.searchParams.set("namespace", namespace);
		url.searchParams.set("limit", "100");
		if (cursor) {
			url.searchParams.set("cursor", cursor);
		}

		const response = await fetch(url, {
			headers: {
				Authorization: `Bearer ${token}`,
			},
		});
		if (!response.ok) {
			const errorBody = await response.text().catch(() => "");
			throw new Error(
				`List actor names failed: ${response.status} ${response.statusText} ${errorBody}`,
			);
		}

		const responseJson = (await response.json()) as ActorNamesResponse;
		names.push(...Object.keys(responseJson.names ?? {}));

		const nextCursor = responseJson.pagination?.cursor ?? undefined;
		if (!nextCursor) {
			return names;
		}
		cursor = nextCursor;
	}
}

async function listActorIdsForName(
	endpoint: string,
	namespace: string,
	name: string,
	token: string,
): Promise<string[]> {
	const actorIds: string[] = [];
	let cursor: string | undefined;

	for (;;) {
		const url = new URL("/actors", endpoint);
		url.searchParams.set("namespace", namespace);
		url.searchParams.set("name", name);
		url.searchParams.set("limit", "100");
		if (cursor) {
			url.searchParams.set("cursor", cursor);
		}

		const response = await fetch(url, {
			headers: {
				Authorization: `Bearer ${token}`,
			},
		});
		if (!response.ok) {
			const errorBody = await response.text().catch(() => "");
			throw new Error(
				`List actors failed for ${name}: ${response.status} ${response.statusText} ${errorBody}`,
			);
		}

		const responseJson = (await response.json()) as ActorListResponse;
		actorIds.push(
			...((responseJson.actors ?? [])
				.map((actor) => actor.actor_id)
				.filter((actorId): actorId is string => !!actorId)),
		);

		const nextCursor = responseJson.pagination?.cursor ?? undefined;
		if (!nextCursor) {
			return actorIds;
		}
		cursor = nextCursor;
	}
}

async function destroyNamespaceActors(
	endpoint: string,
	namespace: string,
	token: string,
): Promise<void> {
	const names = await listAllActorNames(endpoint, namespace, token);
	for (const name of names) {
		const actorIds = await listActorIdsForName(
			endpoint,
			namespace,
			name,
			token,
		);
		await Promise.all(
			actorIds.map(async (actorId) => {
				const url = new URL(`/actors/${actorId}`, endpoint);
				url.searchParams.set("namespace", namespace);

				const response = await fetch(url, {
					method: "DELETE",
					headers: {
						Authorization: `Bearer ${token}`,
					},
				});
				if (response.status === 404) {
					return;
				}
				if (!response.ok) {
					const errorBody = await response.text().catch(() => "");
					throw new Error(
						`Delete actor ${actorId} failed: ${response.status} ${response.statusText} ${errorBody}`,
					);
				}
			}),
		);
	}
}

async function waitForEnvoyCount(
	endpoint: string,
	namespace: string,
	runnerName: string,
	token: string,
	expectAtLeastOne: boolean,
): Promise<void> {
	const envoysUrl = new URL(`${endpoint.replace(/\/$/, "")}/envoys`);
	envoysUrl.searchParams.set("namespace", namespace);
	envoysUrl.searchParams.set("name", runnerName);

	let probeError: unknown;
	for (let attempt = 0; attempt < 150; attempt++) {
		try {
			const envoyResponse = await fetch(envoysUrl, {
				method: "GET",
				headers: {
					Authorization: `Bearer ${token}`,
				},
			});
			if (!envoyResponse.ok) {
				const errorBody = await envoyResponse.text().catch(() => "");
				probeError = new Error(
					`List envoys failed: ${envoyResponse.status} ${envoyResponse.statusText} ${errorBody}`,
				);
			} else {
				const responseJson =
					(await envoyResponse.json()) as {
						envoys?: Array<{ pool_name?: string }>;
					};
				const count =
					responseJson.envoys?.filter(
						(envoy) => envoy.pool_name === runnerName,
					).length ?? 0;
				if (expectAtLeastOne ? count > 0 : count === 0) {
					return;
				}

				probeError = new Error(
					expectAtLeastOne
						? `Envoy ${runnerName} not registered yet`
						: `Envoy ${runnerName} is still connected`,
				);
			}
		} catch (err) {
			probeError = err;
		}

		if (attempt < 149) {
			await new Promise((resolve) => setTimeout(resolve, 100));
		}
	}

	throw probeError;
}

async function ensureSharedEngineRuntime(
	registry: any,
	endpoint: string,
	namespace: string,
	runnerName: string,
	token: string,
): Promise<SharedEngineRuntime> {
	if (!sharedEngineRuntimePromise) {
		sharedEngineRuntimePromise = (async () => {
			const driverConfig = createEngineDriver();

			registry.config.driver = driverConfig;
			registry.config.endpoint = endpoint;
			registry.config.namespace = namespace;
			registry.config.token = token;
			registry.config.envoy = {
				...registry.config.envoy,
				poolName: runnerName,
			};

			const parsedConfig = registry.parseConfig();

			const managerDriver = driverConfig.manager?.(parsedConfig);
			invariant(managerDriver, "missing manager driver");
			const inlineClient = createClientWithDriver(
				managerDriver,
				convertRegistryConfigToClientConfig(parsedConfig),
			);

			const actorDriver = driverConfig.actor(
				parsedConfig,
				managerDriver,
				inlineClient,
			);

			await actorDriver.waitForReady?.();
			await waitForEnvoyCount(
				endpoint,
				namespace,
				runnerName,
				token,
				true,
			);

			return {
				endpoint,
				namespace,
				runnerName,
				token,
				driverConfig,
				actorDriver,
				forceDisconnectKvChannel: async () => {
					const { disconnectKvChannelForCurrentConfig } =
						await import("@/db/native-sqlite");
					return await disconnectKvChannelForCurrentConfig({
						endpoint,
						token,
						namespace,
					});
				},
			};
		})();
	}

	return await sharedEngineRuntimePromise;
}

const driverTestConfig = {
	// Use real timers for engine-runner tests
	useRealTimers: true,
	skip: {
		// The inline client is the same as the remote client driver on Rivet
		inline: true,
	},
	async start() {
		return await createTestRuntime(
			join(__dirname, "../fixtures/driver-test-suite/registry.ts"),
			async (registry) => {
				// Get configuration from environment or use defaults.
				const endpoint =
					process.env.RIVET_ENDPOINT || "http://127.0.0.1:6420";
				const namespaceEndpoint =
					process.env.RIVET_NAMESPACE_ENDPOINT ||
					process.env.RIVET_API_ENDPOINT ||
					endpoint;
				const token = "dev";
				const namespace = await ensureSharedNamespace(
					namespaceEndpoint,
					token,
				);
				const runnerName = await ensureSharedRunnerConfig(
					namespaceEndpoint,
					namespace,
					token,
				);
				const runtime = await ensureSharedEngineRuntime(
					registry,
					endpoint,
					namespace,
					runnerName,
					token,
				);

				return {
					rivetEngine: {
						endpoint: runtime.endpoint,
						namespace: runtime.namespace,
						runnerName: runtime.runnerName,
						token: runtime.token,
					},
						driver: runtime.driverConfig,
						hardCrashActor: async (actorId: string) => {
							await runtime.actorDriver.hardCrashActor?.(actorId);
						},
						hardCrashPreservesData: true,
						forceDisconnectKvChannel: runtime.forceDisconnectKvChannel,
						cleanup: async () => {
							await destroyNamespaceActors(
								namespaceEndpoint,
								namespace,
								token,
							);
						},
					};
			},
		);
	},
} satisfies Omit<DriverTestConfig, "clientType" | "encoding">;

describe.sequential("engine driver", { timeout: 30_000 }, () => {
	runDriverTests(driverTestConfig);

	describe("engine startup kv preload", () => {
		test("wakes actors with envoy-provided preloaded kv", async (c) => {
			const { client } = await setupDriverTest(c, {
				...driverTestConfig,
				clientType: "http",
				encoding: "bare",
			});
			const handle = client.sleep.getOrCreate();

			await handle.getCounts();
			await handle.triggerSleep();

			await vi.waitFor(
				async () => {
					const counts = await handle.getCounts();
					expect(counts.sleepCount).toBeGreaterThanOrEqual(1);
					expect(counts.startCount).toBeGreaterThanOrEqual(2);
				},
				{ timeout: 5_000, interval: 100 },
			);

			const actorId = await handle.resolve();
			const gatewayUrl = await client.sleep
				.getForId(actorId)
				.getGatewayUrl();
			const response = await fetch(`${gatewayUrl}/inspector/metrics`, {
				headers: { Authorization: "Bearer token" },
			});
			expect(response.status).toBe(200);

			const metrics: any = await response.json();
			expect(metrics.startup_is_new.value).toBe(0);
			expect(metrics.startup_internal_preload_kv_entries.value).toBeGreaterThan(0);
			expect(metrics.startup_kv_round_trips.value).toBe(0);
		});
	});
});
