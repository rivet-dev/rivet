import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import { stringifyError } from "@/common/utils";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "@/registry/log";

const CONFIGURE_POOL_MAX_ATTEMPTS = 30;
const CONFIGURE_POOL_RETRY_INTERVAL_MS = 1000;

export async function configureServerlessPool(
	config: RegistryConfig,
): Promise<void> {
	logger().debug({ msg: "configuring serverless pool" });

	for (let attempt = 1; attempt <= CONFIGURE_POOL_MAX_ATTEMPTS; attempt++) {
		try {
			await configureServerlessPoolOnce(config);
			return;
		} catch (error) {
			if (attempt === CONFIGURE_POOL_MAX_ATTEMPTS) {
				logger().error({
					msg: "failed to configure serverless pool, validate endpoint is configured correctly then restart this process",
					errorMessage: stringifyError(error),
				});
				throw error;
			}

			logger().warn({
				msg: "serverless pool configuration failed, retrying",
				attempt,
				maxAttempts: CONFIGURE_POOL_MAX_ATTEMPTS,
				errorMessage: stringifyError(error),
			});
			await new Promise((resolve) =>
				setTimeout(resolve, CONFIGURE_POOL_RETRY_INTERVAL_MS),
			);
		}
	}
}

async function configureServerlessPoolOnce(
	config: RegistryConfig,
): Promise<void> {
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
	const headers = {
		...(config.token ? { "x-rivet-token": config.token } : {}),
		...(customConfig.headers ?? {}),
	};
	const serverlessConfig = {
		serverless: {
			url: customConfig.url,
			headers,
			request_lifespan: customConfig.requestLifespan ?? 15 * 60,
			drain_grace_period: customConfig.drainGracePeriod,
			metadata_poll_interval: customConfig.metadataPollInterval ?? 1000,
			max_runners: 100_000,
			min_runners: 0,
			runners_margin: 0,
			slots_per_runner: 1,
		},
		metadata: customConfig.metadata ?? {},
		drain_on_version_upgrade: customConfig.drainOnVersionUpgrade ?? true,
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
	});
}
