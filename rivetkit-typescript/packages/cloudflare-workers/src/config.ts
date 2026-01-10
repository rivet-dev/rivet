// import type { Client } from "rivetkit";
// import { RunConfigSchema } from "rivetkit/driver-helpers";
// import { z } from "zod";
//
// const ConfigSchemaBase = RunConfigSchema.removeDefault()
// 	.omit({ driver: true, getUpgradeWebSocket: true })
// 	.extend({
// 		/** Path that the Rivet manager API will be mounted. */
// 		managerPath: z.string().optional().default("/rivet"),
//
// 		fetch: z
// 			.custom<
// 				ExportedHandlerFetchHandler<{ RIVET: Client<any> }, unknown>
// 			>()
// 			.optional(),
// 	});
// export const ConfigSchema = ConfigSchemaBase.default(() =>
// 	ConfigSchemaBase.parse({}),
// );
// export type InputConfig = z.input<typeof ConfigSchema>;
// export type Config = z.infer<typeof ConfigSchema>;
