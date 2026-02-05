import { z } from "zod/v4";
import { VERSION } from "@/utils";
import {
	getRivetRunEngineVersion,
	getRivetRunEngine,
	getRivetPublicEndpoint,
	getRivetPublicToken,
} from "@/utils/env-vars";

export const ConfigureRunnerPoolSchema = z
	.object({
		name: z.string().optional(),
		url: z.string(),
		headers: z.record(z.string(), z.string()).optional(),
		maxRunners: z.number().optional(),
		minRunners: z.number().optional(),
		requestLifespan: z.number().optional(),
		runnersMargin: z.number().optional(),
		slotsPerRunner: z.number().optional(),
		metadata: z.record(z.string(), z.unknown()).optional(),
		metadataPollInterval: z.number().optional(),
		drainOnVersionUpgrade: z.boolean().optional(),
	})
	.optional();

export const ServerlessConfigSchema = z.object({
	// MARK: Run Engine
	/**
	 * @experimental
	 *
	 * Downloads and starts the full Rust engine process.
	 * Auto-enabled in development mode when no endpoint is provided.
	 */
	spawnEngine: z.boolean().default(() => getRivetRunEngine()),

	/** @experimental */
	engineVersion: z
		.string()
		.optional()
		.default(() => getRivetRunEngineVersion() ?? VERSION),

	/**
	 * @experimental
	 *
	 * Automatically configure serverless runners in the engine.
	 * Can only be used when runnerKind is "serverless".
	 * If true, uses default configuration. Can also provide custom configuration.
	 */
	configureRunnerPool: ConfigureRunnerPoolSchema.optional(),

	// MARK: Routing
	// TODO: serverlessBasePath? better naming?
	basePath: z.string().optional().default("/api/rivet"),

	// MARK: Public Endpoint Configuration
	/**
	 * The endpoint that clients should connect to.
	 *
	 * This is useful if clients connect to serverless directly
	 * (e.g. `http://localhost:3000/api/rivet`), they will fetch
	 * `http://localhost:3000/api/rivet/metadata` and be redirected to
	 * the public endpoint.
	 *
	 * Supports URL auth syntax for namespace and token:
	 * - `https://namespace:token@api.rivet.dev`
	 * - `https://namespace@api.rivet.dev`
	 *
	 * Auto-determined based on endpoint and NODE_ENV if not specified.
	 *
	 * Can also be set via RIVET_PUBLIC_ENDPOINT environment variable.
	 */
	publicEndpoint: z
		.string()
		.optional()
		.transform((val) => val ?? getRivetPublicEndpoint()),

	/**
	 * Token that clients should use when connecting via the public endpoint.
	 *
	 * Can also be set via RIVET_PUBLIC_TOKEN environment variable.
	 *
	 * Can also be specified in the publicEndpoint URL as `https://namespace:token@host`.
	 */
	publicToken: z
		.string()
		.optional()
		.transform((val) => val ?? getRivetPublicToken()),

	// There is no publicNamespace config option because the frontend and backend
	// cannot use different namespaces. The namespace is extracted from the
	// publicEndpoint URL auth syntax if provided.
});
export type ServerlessConfigInput = z.input<typeof ServerlessConfigSchema>;
export type ServerlessConfig = z.infer<typeof ServerlessConfigSchema>;
