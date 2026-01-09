import type { Client } from "rivetkit";
import { z } from "zod";

const ConfigSchemaBase = z.object({
	/** Path that the Rivet manager API will be mounted. */
	managerPath: z.string().optional().default("/api/rivet"),

	/** Runner key for authentication. */
	runnerKey: z.string().optional(),

	/** Disable the welcome message. */
	noWelcome: z.boolean().optional().default(false),

	fetch: z
		.custom<
			ExportedHandlerFetchHandler<{ RIVET: Client<any> }, unknown>
		>()
		.optional(),
});
export const ConfigSchema = ConfigSchemaBase.default(() =>
	ConfigSchemaBase.parse({}),
);
export type InputConfig = z.input<typeof ConfigSchema>;
export type Config = z.infer<typeof ConfigSchema>;
