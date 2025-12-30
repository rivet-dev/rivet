import { BaseConfigSchema } from "@/registry/config/base";
import {
	getRivetTotalSlots,
	getRivetRunner,
	getRivetRunnerKey,
} from "@/utils/env-vars";
import { z } from "zod";

const ConfigSchemaBase = BaseConfigSchema.extend({
	// Runner fields
	totalSlots: z
		.number()
		.default(() =>
			getRivetTotalSlots() ?? 100000,
		),
	runnerName: z
		.string()
		.default(() => getRivetRunner() ?? "default"),
	runnerKey: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetRunnerKey()),
	// Test-specific fields
	hostname: z
		.string()
		.optional()
		.default(process.env.HOSTNAME ?? "127.0.0.1"),
	port: z
		.number()
		.optional()
		.default(Number.parseInt(process.env.PORT ?? "8080")),
}).transform((config) => {
	// Runner logic:
	// - If endpoint provided: do not start manager server
	// - If no endpoint: start manager server
	const serveManager = config.serveManager ?? !config.endpoint;

	return {
		...config,
		serveManager,
	};
});

export const ConfigSchema = ConfigSchemaBase.default(() =>
	ConfigSchemaBase.parse({}),
);
export type InputConfig = z.input<typeof ConfigSchema>;
