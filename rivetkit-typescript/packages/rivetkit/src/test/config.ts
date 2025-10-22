import { z } from "zod";
import { RunnerConfigSchema } from "@/registry/run-config";

export const ConfigSchema = RunnerConfigSchema.removeDefault()
	.extend({
		hostname: z
			.string()
			.optional()
			.default(process.env.HOSTNAME ?? "127.0.0.1"),
		port: z
			.number()
			.optional()
			.default(Number.parseInt(process.env.PORT ?? "8080")),
	})
	.default({});
export type InputConfig = z.input<typeof ConfigSchema>;
