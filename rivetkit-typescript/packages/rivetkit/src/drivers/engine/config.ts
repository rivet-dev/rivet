import { z } from "zod";
import { ClientConfigSchema } from "@/client/config";
import { getEnvUniversal } from "@/utils";

export const EngingConfigSchema = z
	.object({
		/** Unique key for this runner. Runners connecting a given key will replace any other runner connected with the same key. */
		runnerKey: z
			.string()
			.optional()
			.transform((x) => x ?? getEnvUniversal("RIVET_RUNNER_KEY")),

		/** How many actors this runner can run. */
		totalSlots: z.number().default(100_000),
	})
	// We include the client config since this includes the common properties like endpoint, namespace, etc.
	.merge(ClientConfigSchema)
	.default({});

export type EngineConfig = z.infer<typeof EngingConfigSchema>;
export type EngineConfigInput = z.input<typeof EngingConfigSchema>;
