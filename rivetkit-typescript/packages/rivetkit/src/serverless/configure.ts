import { upsertRunnerConfigForAllDatacenters } from "@/engine-client/runner-config";
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
		const poolName = customConfig.name ?? "default";
		const headers = {
			...(config.token ? { "x-rivet-token": config.token } : {}),
			...(customConfig.headers ?? {}),
		};

		await upsertRunnerConfigForAllDatacenters(config, poolName, {
			serverless: {
				url: customConfig.url,
				headers,
				requestLifespan: customConfig.requestLifespan ?? 15 * 60,
				metadataPollInterval:
					customConfig.metadataPollInterval ?? 1000,
				maxRunners: 100_000,
				minRunners: 0,
				runnersMargin: 0,
				slotsPerRunner: 1,
			},
			metadata: customConfig.metadata ?? {},
			drainOnVersionUpgrade:
				customConfig.drainOnVersionUpgrade ?? true,
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
