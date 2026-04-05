import { z } from "zod/v4";
import { getLogger } from "@/common/log";
import {
	isDev,
	getNodeEnv,
	getRivetPool,
	getRivetTotalSlots,
	getRivetEnvoyVersion,
} from "@/utils/env-vars";

let warnedMissingVersion = false;

export const EnvoyConfigSchema = z.object({
	poolName: z.string().default(() => getRivetPool() ?? "default"),
	version: z.number().default(() => {
		const version = getRivetEnvoyVersion();
		if (version !== undefined) return version;

		if (getNodeEnv() === "production" && !warnedMissingVersion) {
			warnedMissingVersion = true;
			getLogger("rivetkit").error(
				"RIVET_ENVOY_VERSION is not set. Actors will not be versioned, which means they won't be drained on deploy. This is only needed when self-hosting or using a custom envoy (not needed for Rivet Compute). Set this as a build arg in your Dockerfile. See https://rivet.dev/docs/actors/versions",
			);
		}

		return 1;
	}),

	// Deprecated.
	totalSlots: z.number().default(() => getRivetTotalSlots() ?? 100000),
	envoyKey: z.string().optional(),
});
export type EnvoyConfigInput = z.input<typeof EnvoyConfigSchema>;
export type EnvoyConfig = z.infer<typeof EnvoyConfigSchema>;
