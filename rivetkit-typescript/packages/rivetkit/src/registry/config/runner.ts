import { z } from "zod";
import {
	isDev,
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerKey,
} from "@/utils/env-vars";

export const RunnerConfigSchema = z.object({
	// MARK: Runner
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),
	runnerKey: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetRunnerKey()),
});
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
