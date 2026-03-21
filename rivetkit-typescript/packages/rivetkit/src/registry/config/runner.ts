import { z } from "zod/v4";
import { getLogger } from "@/common/log";
import {
	isDev,
	getNodeEnv,
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerVersion,
} from "@/utils/env-vars";

let warnedMissingVersion = false;

export const RunnerConfigSchema = z.object({
	// MARK: Runner
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),
	// Deprecated.
	runnerKey: z
		.string()
		.optional(),
	version: z.number().default(() => {
		const version = getRivetRunnerVersion();
		if (version !== undefined) return version;

		if (getNodeEnv() === "production" && !warnedMissingVersion) {
			warnedMissingVersion = true;
			getLogger("rivetkit").error(
				"RIVET_RUNNER_VERSION is not set. Actors will not be versioned, which means they won't be drained on deploy. This is only needed when self-hosting or using a custom runner (not needed for Rivet Compute). Set this as a build arg in your Dockerfile. See https://rivet.dev/docs/actors/versions",
			);
		}

		return 1;
	}),
});
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
