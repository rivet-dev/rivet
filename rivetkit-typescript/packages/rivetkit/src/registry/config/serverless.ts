import { z } from "zod";
import { VERSION } from "@/utils";
import { getRivetRunEngineVersion, getRivetRunEngine } from "@/utils/env-vars";

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

	/**
	 * The endpoint that clients should connect to.
	 *
	 * This is useful if clients connect to serverless directly
	 * (e.g. `http://localhost:3000/api/rivet`), they will fetch
	 * `http://localhost:3000/api/rivet/metadata` and be redirected to
	 * the advertised endpoint.
	 *
	 * Auto-determined based on endpoint and NODE_ENV if not specified.
	 */
	advertiseEndpoint: z.string().optional(),
});
export type ServerlessConfigInput = z.input<typeof ServerlessConfigSchema>;
export type ServerlessConfig = z.infer<typeof ServerlessConfigSchema>;
