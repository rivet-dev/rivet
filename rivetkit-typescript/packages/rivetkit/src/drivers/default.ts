import { UserError } from "@/actor/errors";
import { loggerWithoutContext } from "@/actor/log";
import { createEngineDriver } from "@/drivers/engine/mod";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import type { DriverConfig, RunnerConfig } from "@/registry/run-config";

/**
 * Chooses the appropriate driver based on the run configuration.
 */
export async function chooseDefaultDriver(
	runConfig: RunnerConfig,
): Promise<DriverConfig> {
	if (runConfig.endpoint && runConfig.driver) {
		throw new UserError(
			"Cannot specify both 'endpoint' and 'driver' in configuration",
		);
	}

	if (runConfig.driver) {
		return runConfig.driver;
	}

	if (
		runConfig.endpoint ||
		runConfig.token ||
		runConfig.runnerKind === "serverless"
	) {
		loggerWithoutContext().debug({
			msg: "using rivet engine driver",
			endpoint: runConfig.endpoint,
		});
		return createEngineDriver();
	}

	loggerWithoutContext().debug({ msg: "using default file system driver" });
	return await createFileSystemOrMemoryDriver(true);
}
