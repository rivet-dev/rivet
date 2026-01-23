import { RegistryConfig } from "@/registry/config";
import { logger } from "./log";
import invariant from "invariant";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import {
	getDatacenters,
	updateRunnerConfig,
} from "@/remote-manager-driver/api-endpoints";

export async function configureServerlessRunner(
	config: RegistryConfig,
): Promise<void> {
	logger().debug("configuring serverless runner");

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
		const customConfig = config.serverless.configureRunnerPool;
		invariant(customConfig, "configureRunnerPool should exist");

		const clientConfig = convertRegistryConfigToClientConfig(config);

		// Fetch all datacenters
		logger().debug({
			msg: "fetching datacenters",
			endpoint: config.endpoint,
		});
		const dcsRes = await getDatacenters(clientConfig);

		// Build the request body
		const runnerName = customConfig.name ?? "default";
		logger().debug({
			msg: "configuring serverless runner",
			runnerName,
			namespace: config.namespace,
		});
		const serverlessConfig = {
			serverless: {
				url: customConfig.url,
				headers: customConfig.headers ?? {},
				max_runners: customConfig.maxRunners ?? 1000,
				min_runners: customConfig.minRunners ?? 0,
				request_lifespan: customConfig.requestLifespan ?? 15 * 60,
				runners_margin: customConfig.runnersMargin ?? 0,
				slots_per_runner: customConfig.slotsPerRunner ?? 1,
				metadata_poll_interval: customConfig.metadataPollInterval ?? 1000,
			},
			metadata: customConfig.metadata ?? {},
			drain_on_version_upgrade: customConfig.drainOnVersionUpgrade ?? true,
			metadataPollInterval: customConfig.metadataPollInterval ?? 1000,
		};
		await updateRunnerConfig(clientConfig, runnerName, {
			datacenters: Object.fromEntries(
				dcsRes.datacenters.map((dc) => [dc.name, serverlessConfig]),
			),
		});

		logger().info({
			msg: "serverless runner configured successfully",
			runnerName,
			namespace: config.namespace,
		});
	} catch (error) {
		logger().error({
			msg: "failed to configure serverless runner, validate endpoint is configured correctly then restart this process",
			error,
		});

		// Don't throw, allow the runner to continue
	}
}
