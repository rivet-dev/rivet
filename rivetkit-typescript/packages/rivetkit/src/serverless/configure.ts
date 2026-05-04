import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import { stringifyError } from "@/common/utils";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "@/registry/log";

const DEFAULT_CONFIGURE_TIMEOUT_MS = 60_000;
const CONFIGURE_RETRY_DELAY_MS = 1_000;

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function configureTimeoutMs() {
	const value = process.env.RIVET_SERVERLESS_CONFIGURE_TIMEOUT_MS;
	if (value === undefined || value === "") return DEFAULT_CONFIGURE_TIMEOUT_MS;

	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed < 0) {
		throw new Error("RIVET_SERVERLESS_CONFIGURE_TIMEOUT_MS must be a finite non-negative number");
	}

	return parsed;
}

export async function configureServerlessPool(
	config: RegistryConfig,
): Promise<void> {
	logger().debug({ msg: "configuring serverless pool" });

	const startedAt = Date.now();
	const timeoutMs = configureTimeoutMs();
	let attempts = 0;
	let lastError: unknown;

	while (Date.now() - startedAt <= timeoutMs) {
		attempts += 1;
		try {
			if (!config.namespace) {
				throw new Error("namespace is required for serverless configuration");
			}
			if (!config.endpoint) {
				throw new Error("endpoint is required for serverless configuration");
			}
			if (!config.configurePool) {
				throw new Error("configurePool is required for serverless configuration");
			}

			const customConfig = config.configurePool;
			const clientConfig = convertRegistryConfigToClientConfig(config);
			const dcsRes = await getDatacenters(clientConfig);
			const poolName = customConfig.name ?? "default";
			const serverlessToken = config.token ?? config.publicToken;
			const headers = {
				...(serverlessToken ? { "x-rivet-token": serverlessToken } : {}),
				...(customConfig.headers ?? {}),
			};
			const serverlessConfig = {
				serverless: {
					url: customConfig.url,
					headers,
					request_lifespan: customConfig.requestLifespan ?? 15 * 60,
					drain_grace_period: customConfig.drainGracePeriod,
					metadata_poll_interval:
						customConfig.metadataPollInterval ?? 1000,
					max_runners: 100_000,
					min_runners: 0,
					runners_margin: 0,
					slots_per_runner: 1,
				},
				metadata: customConfig.metadata ?? {},
				drain_on_version_upgrade:
					customConfig.drainOnVersionUpgrade ?? true,
			};

			await updateRunnerConfig(clientConfig, poolName, {
				datacenters: Object.fromEntries(
					dcsRes.datacenters.map((dc) => [dc.name, serverlessConfig]),
				),
			});

			logger().info({
				msg: "serverless pool configured successfully",
				poolName,
				namespace: config.namespace,
				attempts,
			});
			return;
		} catch (error) {
			lastError = error;
			logger().warn({
				msg: "serverless pool configuration attempt failed",
				attempts,
				error: stringifyError(error),
			});
			await sleep(CONFIGURE_RETRY_DELAY_MS);
		}
	}

	logger().error({
		msg: "failed to configure serverless pool, validate endpoint is configured correctly then restart this process",
		attempts,
		error: stringifyError(lastError),
	});
	throw lastError;
}
