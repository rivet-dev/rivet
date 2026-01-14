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
import type { RegistryConfig } from "@/registry/config";
import { tryParseEndpoint } from "@/utils/endpoint-parser";

/**
 * Gets the default endpoint for the client.
 *
 * In browser: uses current origin + /api/rivet
 *
 * Server-side: uses localhost:6420
 */
function getDefaultEndpoint(): string {
	if (typeof window !== "undefined" && window.location?.origin) {
		return `${window.location.origin}/api/rivet`;
	}
	return "http://127.0.0.1:6420";
}

/**
 * Base client config schema without transforms so it can be merged in to other schemas.
 */
export const ClientConfigSchemaBase = z.object({
	/**
	 * Endpoint to connect to for Rivet Engine or RivetKit manager API.
	 *
	 * Supports URL auth syntax for namespace and token:
	 * - `https://namespace:token@api.rivet.dev`
	 * - `https://namespace@api.rivet.dev`
	 *
	 * Can also be set via RIVET_ENDPOINT environment variables.
	 *
	 * Defaults to current origin + /api/rivet in browser, or localhost:6420 server-side.
	 */
	endpoint: z
		.string()
		.optional()
		.transform(
			(val) =>
				val ??
				getRivetEngine() ??
				getRivetEndpoint() ??
				getDefaultEndpoint(),
		),

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

	/** Whether to enable RivetKit Devtools integration. */
	devtools: z
		.boolean()
		.default(
			() =>
				typeof window !== "undefined" &&
				window?.location?.hostname === "localhost",
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
		runnerName: config.runner.runnerName,
		headers: config.headers,
		encoding: "bare",
		getUpgradeWebSocket: undefined,
		// We don't need health checks for internal clients
		disableMetadataLookup: true,
		devtools:
			typeof window !== "undefined" &&
			window?.location?.hostname === "localhost",
	};
}
