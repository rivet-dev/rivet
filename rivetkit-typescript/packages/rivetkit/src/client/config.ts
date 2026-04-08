import z from "zod/v4";
import { EncodingSchema } from "@/actor/protocol/serde";
import type { RegistryConfig } from "@/registry/config";
import type { GetUpgradeWebSocket } from "@/utils";
import { tryParseEndpoint } from "@/utils/endpoint-parser";
import {
	getRivetEndpoint,
	getRivetEngine,
	getRivetNamespace,
	getRivetPool,
	getRivetToken,
} from "@/utils/env-vars";

const DEFAULT_ENDPOINT = "http://localhost:6420";
export const DEFAULT_MAX_QUERY_INPUT_SIZE = 4 * 1024;

let hasWarnedMissingEndpoint = false;

/**
 * Base client config schema without transforms so it can be merged in to other schemas.
 */
export const ClientConfigSchemaBase = z.object({
	/**
	 * Endpoint to connect to for Rivet Engine or the local RivetKit runtime API.
	 *
	 * Supports URL auth syntax for namespace and token:
	 * - `https://namespace:token@api.rivet.dev`
	 * - `https://namespace@api.rivet.dev`
	 *
	 * Can also be set via RIVET_ENDPOINT environment variables.
	 *
	 * Defaults to http://localhost:6420.
	 */
	endpoint: z
		.string()
		.optional()
		.transform((val) => {
			const resolved = val ?? getRivetEngine() ?? getRivetEndpoint();
			if (!resolved && !hasWarnedMissingEndpoint) {
				hasWarnedMissingEndpoint = true;
				console.warn(
					`[rivetkit] No endpoint provided to client. Defaulting to ${DEFAULT_ENDPOINT}. ` +
					`Starting in 2.2.0, an explicit endpoint will be required. ` +
					`Pass an endpoint to createClient() or createRivetKit(), ` +
					`or set the RIVET_ENDPOINT environment variable.`,
				);
			}
			return resolved ?? DEFAULT_ENDPOINT;
		}),

	/** Token to use to authenticate with the API. */
	token: z
		.string()
		.optional()
		.transform((val) => val ?? getRivetToken()),

	/** Namespace to connect to. */
	namespace: z
		.string()
		.optional()
		.transform((val) => val ?? getRivetNamespace()),

	/** Name of the envoy pool. This is used to group together envoys in to different pools. */
	poolName: z.string().default(() => getRivetPool() ?? "default"),

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

	/**
	 * Maximum serialized query input size in bytes before base64url encoding.
	 *
	 * This applies to query-backed `getOrCreate()` and `create()` gateway URLs.
	 */
	maxInputSize: z
		.number()
		.int()
		.positive()
		.default(DEFAULT_MAX_QUERY_INPUT_SIZE),

	/** Whether to enable RivetKit Devtools integration. */
	devtools: z
		.boolean()
		.default(
			() =>
				typeof window !== "undefined" &&
				(window?.location?.hostname === "127.0.0.1" ||
					window.location?.hostname === "localhost"),
		),
});

export const ClientConfigSchema = ClientConfigSchemaBase.transform(
	(config, ctx) => transformClientConfig(config, ctx),
);

export type ClientConfig = z.infer<typeof ClientConfigSchema>;

export type ClientConfigInput = z.input<typeof ClientConfigSchema>;

export function transformClientConfig(
	config: z.infer<typeof ClientConfigSchemaBase>,
	ctx: z.RefinementCtx,
) {
	const parsedEndpoint = tryParseEndpoint(ctx, {
		endpoint: config.endpoint,
		path: ["endpoint"],
		namespace: config.namespace,
		token: config.token,
	});

	return {
		...config,
		endpoint: parsedEndpoint?.endpoint,
		namespace: parsedEndpoint?.namespace ?? config.namespace ?? "default",
		token: parsedEndpoint?.token ?? config.token,
	};
}

/**
 * Converts a base config in to a client config.
 *
 * The base config does not include all of the properties of the client config,
 * so this converts the subset of properties in to the client config.
 *
 * Note: We construct the object directly rather than using ClientConfigSchema.parse()
 * because RegistryConfig has already transformed the endpoint, namespace, and token.
 * Re-parsing would attempt to extract namespace/token from the endpoint URL again.
 */
export function convertRegistryConfigToClientConfig(
	config: RegistryConfig,
): ClientConfig {
	return {
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		poolName: config.envoy.poolName,
		headers: config.headers,
		encoding: "bare",
		getUpgradeWebSocket: undefined,
		// We don't need health checks for internal clients
		disableMetadataLookup: true,
		maxInputSize: DEFAULT_MAX_QUERY_INPUT_SIZE,
		devtools:
			typeof window !== "undefined" &&
			(window?.location?.hostname === "127.0.0.1" ||
				window?.location?.hostname === "localhost"),
	};
}
