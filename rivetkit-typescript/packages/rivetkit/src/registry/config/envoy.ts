import { z } from "zod/v4";
import {
	getRivetEnvoyVersion,
	getRivetPool,
	getRivetTotalSlots,
} from "@/utils/env-vars";

export const EnvoyConfigSchema = z.object({
	poolName: z.string().default(() => getRivetPool() ?? "default"),
	version: z.number().default(() => getRivetEnvoyVersion() ?? 1),

	// Deprecated.
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	envoyKey: z.string().optional(),
});
export type EnvoyConfigInput = z.input<typeof EnvoyConfigSchema>;
export type EnvoyConfig = z.infer<typeof EnvoyConfigSchema>;
