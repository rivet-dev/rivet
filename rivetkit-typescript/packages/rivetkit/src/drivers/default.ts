import { UserError } from "@/actor/errors";
import { loggerWithoutContext } from "@/actor/log";
import { createEngineDriver } from "@/drivers/engine/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { DriverConfig, RegistryConfig } from "@/registry/config";
import { hasNodeDependencies } from "@/utils/node";

/**
 * Chooses the appropriate driver based on the run configuration.
 */
export function chooseDefaultDriver(config: RegistryConfig): DriverConfig {
	if (config.endpoint && config.driver) {
		throw new UserError(
			"Cannot specify both 'endpoint' and 'driver' in configuration",
		);
	}

	if (config.driver) {
		return config.driver;
	}

	if (config.endpoint || config.token) {
		loggerWithoutContext().debug({
			msg: "using rivet engine driver",
			endpoint: config.endpoint,
		});
		return createEngineDriver();
	}

	// In edge environments (Convex, Cloudflare Workers), file system driver
	// is not available. Use engine driver for serverless runners - the actual
	// endpoint will be configured dynamically when Rivet Cloud calls /start.
	if (!hasNodeDependencies()) {
		loggerWithoutContext().debug({
			msg: "using engine driver for edge environment (serverless)",
		});
		return createEngineDriver();
	}

	loggerWithoutContext().debug({ msg: "using default file system driver" });
	return createFileSystemOrMemoryDriver(true);
}
