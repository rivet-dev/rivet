import { z } from "zod";
import {
	isDev,
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerKey,
} from "@/utils/env-vars";
import { BaseConfigSchema } from "./base";

export const RunnerConfigSchema = BaseConfigSchema.extend({
	// MARK: Runner
	totalSlots: z
		.number()
		.default(() => getRivetTotalSlots() ?? 100000),
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),
	runnerKey: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetRunnerKey()),
})
	.superRefine((config, ctx) => {
		if (config.endpoint && config.serveManager) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both endpoint and serveManager",
			});
		}
	})
	.transform((config) => {
		const isDevEnv = isDev();

		// Runner logic:
		// - If endpoint provided: do not start manager server
		// - If dev mode without endpoint: start manager server
		// - If prod mode without endpoint: do not start manager server
		let serveManager: boolean;
		if (config.endpoint) {
			serveManager = config.serveManager ?? false;
		} else if (isDevEnv) {
			serveManager = config.serveManager ?? true;
		} else {
			serveManager = config.serveManager ?? false;
		}

		// If endpoint is set, we'll use engine driver - disable manager inspector
		const willUseEngine = !!config.endpoint;
		const inspector = willUseEngine
			? { ...config.inspector, enabled: { manager: false, actor: true } }
			: config.inspector;

		return {
			...config,
			serveManager,
			inspector,
		};
	});
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
