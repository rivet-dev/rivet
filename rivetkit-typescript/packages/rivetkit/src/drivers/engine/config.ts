import { z } from "zod";
import { ClientConfigSchema } from "@/client/config";
import { getRivetRunnerKey } from "@/utils/env-vars";

const EngineConfigSchemaBase = z
	.object({
		/** Unique key for this runner. Runners connecting a given key will replace any other runner connected with the same key. */
		runnerKey: z
			.string()
			.optional()
			.transform((x) => x ?? getRivetRunnerKey()),

		/** How many actors this runner can run. */
		totalSlots: z.number().default(100_000),
	})
	// We include the client config since this includes the common properties like endpoint, namespace, etc.
	.merge(ClientConfigSchema);

export const EngingConfigSchema = EngineConfigSchemaBase.default(() =>
	EngineConfigSchemaBase.parse({}),
);

export type EngineConfig = z.infer<typeof EngingConfigSchema>;
export type EngineConfigInput = z.input<typeof EngingConfigSchema>;
