import z from "zod";
import { EncodingSchema } from "@/actor/protocol/serde";
import { type GetUpgradeWebSocket, getEnvUniversal } from "@/utils";

export const ClientConfigSchema = z.object({
	/** Endpoint to connect to for Rivet Engine or RivetKit manager API. */
	endpoint: z
		.string()
		.optional()
		.transform(
			(x) =>
				x ??
				getEnvUniversal("RIVET_ENGINE") ??
				getEnvUniversal("RIVET_ENDPOINT"),
		),

	/** Token to use to authenticate with the API. */
	token: z
		.string()
		.optional()
		.transform((x) => x ?? getEnvUniversal("RIVET_TOKEN")),

	/** Namespace to connect to. */
	namespace: z
		.string()
		.default(() => getEnvUniversal("RIVET_NAMESPACE") ?? "default"),

	/** Name of the runner. This is used to group together runners in to different pools. */
	runnerName: z
		.string()
		.default(() => getEnvUniversal("RIVET_RUNNER") ?? "default"),

	encoding: EncodingSchema.default("bare"),

	headers: z.record(z.string()).optional().default({}),

	// See RunConfig.getUpgradeWebSocket
	getUpgradeWebSocket: z.custom<GetUpgradeWebSocket>().optional(),

	/** Whether to automatically perform health checks when the client is created. */
	disableMetadataLookup: z.boolean().optional().default(false),

	/** Whether to enable RivetKit Devtools integration. */
	devtools: z
		.boolean()
		.default(
			() =>
				typeof globalThis.window !== "undefined" &&
				window?.location?.hostname === "localhost",
		),
});

export type ClientConfig = z.infer<typeof ClientConfigSchema>;

export type ClientConfigInput = z.input<typeof ClientConfigSchema>;
