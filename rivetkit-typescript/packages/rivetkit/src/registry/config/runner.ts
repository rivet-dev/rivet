import { z } from "zod";
import {
	isDev,
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerKey,
	getRivetRunnerVersion,
} from "@/utils/env-vars";

export const RunnerConfigSchema = z.object({
	// MARK: Runner
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),
	runnerKey: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetRunnerKey()),
	version: z.number().default(() => getRivetRunnerVersion() ?? 1),
});
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
