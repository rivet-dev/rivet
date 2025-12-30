import { UserError } from "@/actor/errors";
import { loggerWithoutContext } from "@/actor/log";
import { createEngineDriver } from "@/drivers/engine/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import { DriverConfig, BaseConfig } from "@/registry/config/base";

/**
 * Chooses the appropriate driver based on the run configuration.
 */
export function chooseDefaultDriver(
	runConfig: BaseConfig,
): DriverConfig {
	if (runConfig.endpoint && runConfig.driver) {
		throw new UserError(
			"Cannot specify both 'endpoint' and 'driver' in configuration",
		);
	}

	if (runConfig.driver) {
		return runConfig.driver;
	}

	if (runConfig.endpoint || runConfig.token) {
		loggerWithoutContext().debug({
			msg: "using rivet engine driver",
			endpoint: runConfig.endpoint,
		});
		return createEngineDriver();
	}

	loggerWithoutContext().debug({ msg: "using default file system driver" });
	return createFileSystemOrMemoryDriver(true);
}
