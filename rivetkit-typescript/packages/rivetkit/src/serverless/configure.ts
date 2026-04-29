import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "@/registry/log";

export async function configureServerlessPool(
	config: RegistryConfig,
): Promise<void> {
	logger().debug({ msg: "configuring serverless pool" });

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
		});
	} catch (error) {
		logger().error({
			msg: "failed to configure serverless pool, validate endpoint is configured correctly then restart this process",
			error,
		});
	}
}
