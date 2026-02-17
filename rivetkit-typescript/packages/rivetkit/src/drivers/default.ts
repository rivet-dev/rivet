import { UserError } from "@/actor/errors";
import { loggerWithoutContext } from "@/actor/log";
import { createEngineDriver } from "@/drivers/engine/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { DriverConfig, RegistryConfig } from "@/registry/config";

/**
 * Chooses the appropriate driver based on the run configuration.
 */
export function chooseDefaultDriver(
	config: RegistryConfig,
): DriverConfig {
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

	loggerWithoutContext().debug({
		msg: "using default file system driver",
		storagePath: config.storagePath,
	});
	return createFileSystemOrMemoryDriver(true, {
		path: config.storagePath,
	});
}
