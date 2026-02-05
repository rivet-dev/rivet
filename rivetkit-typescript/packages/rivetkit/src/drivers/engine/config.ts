import { z } from "zod/v4";
import {
	ClientConfigSchemaBase,
	transformClientConfig,
} from "@/client/config";
import { getRivetRunnerKey } from "@/utils/env-vars";

/**
 * Base engine config schema without transforms so it can be merged in to other schemas.
 *
 * We include the client config since this includes the common properties like endpoint, namespace, etc.
 */
export const EngineConfigSchemaBase = ClientConfigSchemaBase.extend({
	/** Unique key for this runner. Runners connecting a given key will replace any other runner connected with the same key. */
	runnerKey: z
		.string()
		.optional()
		.transform((val) => val ?? getRivetRunnerKey()),

	/** How many actors this runner can run. */
	totalSlots: z.number().default(100_000),
});

const EngineConfigSchemaTransformed = EngineConfigSchemaBase.transform(
	(config, ctx) => transformEngineConfig(config, ctx),
);

export const EngineConfigSchema = EngineConfigSchemaTransformed.default(() =>
	EngineConfigSchemaTransformed.parse({}),
);

export type EngineConfig = z.infer<typeof EngineConfigSchema>;
export type EngineConfigInput = z.input<typeof EngineConfigSchema>;

export function transformEngineConfig(
	config: z.infer<typeof EngineConfigSchemaBase>,
	ctx: z.RefinementCtx,
) {
	return {
		...transformClientConfig(config, ctx),
		runnerKey: config.runnerKey,
	};
}
