import invariant from "invariant";
import { z } from "zod";
import { VERSION } from "@/utils";
import {
	getRivetRunEngineVersion,
	isDev,
	getRivetRunEngine,
} from "@/utils/env-vars";
import { BaseConfigSchema } from "./base";

export const ServerlessConfigSchema = BaseConfigSchema.extend({

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
	configureRunnerPool: z
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
		.optional(),

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
})
	.superRefine((config, ctx) => {
		const isDevEnv = isDev();

		// Can't spawn engine AND connect to remote endpoint
		if (config.spawnEngine && config.endpoint) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both spawnEngine and endpoint",
			});
		}

		// spawnEngine and serveManager are mutually exclusive
		if (config.spawnEngine && config.serveManager) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both spawnEngine and serveManager",
			});
		}

		// configureRunnerPool requires an engine (via endpoint or spawnEngine)
		if (
			config.configureRunnerPool &&
			!config.endpoint &&
			!config.spawnEngine
		) {
			ctx.addIssue({
				code: "custom",
				message:
					"configureRunnerPool requires either endpoint or spawnEngine",
			});
		}

		// advertiseEndpoint required in production without endpoint
		if (!isDevEnv && !config.endpoint && !config.advertiseEndpoint) {
			ctx.addIssue({
				code: "custom",
				message:
					"advertiseEndpoint is required in production mode without endpoint",
				path: ["advertiseEndpoint"],
			});
		}
	})
	.transform((config) => {
		const isDevEnv = isDev();

		let serveManager: boolean;
		let advertiseEndpoint: string;

		if (config.endpoint) {
			// Remote endpoint provided:
			// - Do not start manager server
			// - Redirect clients to remote endpoint
			serveManager = config.serveManager ?? false;
			advertiseEndpoint = config.advertiseEndpoint ?? config.endpoint;
		} else if (isDevEnv) {
			// Development mode, no endpoint:
			// - Start manager server
			// - Redirect clients to local server
			serveManager = config.serveManager ?? true;
			advertiseEndpoint =
				config.advertiseEndpoint ??
				`http://localhost:${config.managerPort}`;
		} else {
			// Production mode, no endpoint:
			// - Do not start manager server
			// - Use file system driver
			serveManager = config.serveManager ?? false;
			invariant(
				config.advertiseEndpoint,
				"advertiseEndpoint is required in production mode without endpoint",
			);
			advertiseEndpoint = config.advertiseEndpoint;
		}

		// If endpoint is set or spawning engine, we'll use engine driver - disable manager inspector
		const willUseEngine = !!config.endpoint || config.spawnEngine;
		const inspector = willUseEngine
			? { ...config.inspector, enabled: { manager: false, actor: true } }
			: config.inspector;

		return {
			...config,
			serveManager,
			advertiseEndpoint,
			inspector,
		};
	});
export type ServerlessConfigInput = z.input<typeof ServerlessConfigSchema>;
export type ServerlessConfig = z.infer<typeof ServerlessConfigSchema>;
