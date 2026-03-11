import { z } from "zod/v4";
import {
	isDev,
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerVersion,
} from "@/utils/env-vars";

export const RunnerConfigSchema = z.object({
	// MARK: Runner
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),
	// Deprecated.
	runnerKey: z
		.string()
		.optional(),
	version: z.number().default(() => getRivetRunnerVersion() ?? 1),
});
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
