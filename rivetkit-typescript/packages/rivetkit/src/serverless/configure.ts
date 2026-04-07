import { RegistryConfig } from "@/registry/config";
import { logger } from "./log";
import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/engine-client/api-endpoints";

export async function configureServerlessPool(
	config: RegistryConfig,
): Promise<void> {
	logger().debug("configuring serverless pool");

	try {
		// Ensure we have required config values
		if (!config.namespace) {
			throw new Error(
				"namespace is required for serverless configuration",
			);
		}
		if (!config.endpoint) {
			throw new Error(
				"endpoint is required for serverless configuration",
			);
		}

		// Prepare the configuration
		const customConfig = config.serverless.configurePool;
		invariant(customConfig, "configurePool should exist");

		const clientConfig = convertRegistryConfigToClientConfig(config);

		// Fetch all datacenters
		logger().debug({
			msg: "fetching datacenters",
			endpoint: config.endpoint,
		});
		const dcsRes = await getDatacenters(clientConfig);

		// Build the request body
		const poolName = customConfig.name ?? "default";
		logger().debug({
			msg: "configuring serverless pool",
			poolName,
			namespace: config.namespace,
		});
		const serverlessConfig = {
			serverless: {
				url: customConfig.url,
				headers: customConfig.headers ?? {},
				request_lifespan: customConfig.requestLifespan ?? 15 * 60,
				max_concurrent_actors: customConfig.maxConcurrentActors ?? 100_000,
				metadata_poll_interval:
					customConfig.metadataPollInterval ?? 1000,

				max_runners: customConfig.maxRunners ?? 100_000,
				min_runners: customConfig.minRunners ?? 0,
				runners_margin: customConfig.runnersMargin ?? 0,
				slots_per_runner: customConfig.slotsPerRunner ?? 1,
			},
			metadata: customConfig.metadata ?? {},
			drain_on_version_upgrade:
				customConfig.drainOnVersionUpgrade ?? true,
			metadataPollInterval: customConfig.metadataPollInterval ?? 1000,
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

		// Don't throw, allow the envoy to continue
	}
}
