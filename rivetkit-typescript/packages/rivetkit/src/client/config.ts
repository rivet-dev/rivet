import z from "zod";
import { EncodingSchema } from "@/actor/protocol/serde";
import { type GetUpgradeWebSocket } from "@/utils";
import {
	getRivetEngine,
	getRivetEndpoint,
	getRivetToken,
	getRivetNamespace,
	getRivetRunner,
} from "@/utils/env-vars";
import { BaseConfig } from "@/registry/config/base";

export const ClientConfigSchema = z.object({
	/** Endpoint to connect to for Rivet Engine or RivetKit manager API. */
	endpoint: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetEngine() ?? getRivetEndpoint()),

	/** Token to use to authenticate with the API. */
	token: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetToken()),

	/** Namespace to connect to. */
	namespace: z.string().default(() => getRivetNamespace() ?? "default"),

	/** Name of the runner. This is used to group together runners in to different pools. */
	runnerName: z.string().default(() => getRivetRunner() ?? "default"),

	encoding: EncodingSchema.default("bare"),

	headers: z
		.record(z.string(), z.string())
		.optional()
		.default(() => ({})),

	// See RunConfig.getUpgradeWebSocket
	//
	// This is required in the client config in order to support
	// `proxyWebSocket`
	getUpgradeWebSocket: z.custom<GetUpgradeWebSocket>().optional(),

	/** Whether to automatically perform health checks when the client is created. */
	disableMetadataLookup: z.boolean().optional().default(false),
});

export type ClientConfig = z.infer<typeof ClientConfigSchema>;

export type ClientConfigInput = z.input<typeof ClientConfigSchema>;

/**
 * Converts a base config in to a client config.
 *
 * The base config does not include all of the properties of the client config,
 * so this converts the subset of properties in to the client config.
 */
export function convertBaseConfigToClientConfig(
	config: BaseConfig,
): ClientConfig {
	return ClientConfigSchema.parse({
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		// TODO: We may need to configure the runner name, TBD how
		// runnerName: config.runnerName,
		headers: config.headers,
		// We don't need health checks for internal clients
		disableMetadataLookup: true,
	});
}
