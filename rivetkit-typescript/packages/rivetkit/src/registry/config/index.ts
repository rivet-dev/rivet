import { z } from "zod";
import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";
import { type Logger, LogLevelSchema } from "@/common/log";
import { ENGINE_ENDPOINT } from "@/engine-process/constants";
import { InspectorConfigSchema } from "@/inspector/config";
import { tryParseEndpoint } from "@/utils/endpoint-parser";
import {
	getRivetEndpoint,
	getRivetEngine,
	getRivetNamespace,
	getRivetToken,
	isDev,
} from "@/utils/env-vars";
import { type DriverConfig, DriverConfigSchema } from "./driver";
import { RunnerConfigSchema } from "./runner";
import { ServerlessConfigSchema } from "./serverless";
import { DeepReadonly } from "@/utils";

export { DriverConfigSchema, type DriverConfig };

export const ActorsSchema = z.record(
	z.string(),
	z.custom<ActorDefinition<any, any, any, any, any, any, any>>(),
);
export type RegistryActors = z.infer<typeof ActorsSchema>;

export const TestConfigSchema = z.object({ enabled: z.boolean() });
export type TestConfig = z.infer<typeof TestConfigSchema>;

// TODO: Add sane defaults for NODE_ENV=development
export const RegistryConfigSchema = z
	.object({
		// MARK: Actors
		use: z.record(z.string(), z.custom<AnyActorDefinition>()),

		// TODO: Find a better way of passing around the test config
		/**
		 * Test configuration.
		 *
		 * DO NOT MANUALLY ENABLE. THIS IS USED INTERNALLY.
		 * @internal
		 **/
		test: TestConfigSchema.optional().default({ enabled: false }),

		// MARK: Driver
		driver: DriverConfigSchema.optional(),

		// MARK: Networking
		/** @experimental */
		maxIncomingMessageSize: z.number().optional().default(65_536),

		/** @experimental */
		maxOutgoingMessageSize: z.number().optional().default(1_048_576),

		// MARK: Runtime
		/**
		 * @experimental
		 *
		 * Disable welcome message.
		 * */
		noWelcome: z.boolean().optional().default(false),

		/**
		 * @experimental
		 * */
		logging: z
			.object({
				baseLogger: z.custom<Logger>().optional(),
				level: LogLevelSchema.optional(),
			})
			.optional()
			.default(() => ({})),

		// MARK: Routing
		// // This is a function to allow for lazy configuration of upgradeWebSocket on the
		// // fly. This is required since the dependencies that upgradeWebSocket
		// // (specifically Node.js) can sometimes only be specified after the router is
		// // created or must be imported async using `await import(...)`
		// getUpgradeWebSocket: z.custom<GetUpgradeWebSocket>().optional(),

		// MARK: Runner Configuration
		/**
		 * Endpoint to connect to for Rivet Engine.
		 *
		 * Supports URL auth syntax for namespace and token:
		 * - `https://namespace:token@api.rivet.dev`
		 * - `https://namespace@api.rivet.dev`
		 *
		 * Can also be set via RIVET_ENDPOINT environment variables.
		 */
		endpoint: z
			.string()
			.optional()
			.transform((val) => val ?? getRivetEngine() ?? getRivetEndpoint()),
		token: z
			.string()
			.optional()
			.transform((val) => val ?? getRivetToken()),
		namespace: z
			.string()
			.optional()
			.transform((val) => val ?? getRivetNamespace()),
		headers: z.record(z.string(), z.string()).optional().default({}),

		// MARK: Client
		// TODO:
		// client: ClientConfigSchema.optional(),

		// MARK: Manager
		/**
		 * Whether to start the local manager server.
		 * Auto-determined based on endpoint and NODE_ENV if not specified.
		 */
		serveManager: z.boolean().optional(),
		/**
		 * @experimental
		 *
		 * Base path for the manager API. This is used to prefix all routes.
		 * For example, if the base path is `/foo`, then the route `/actors`
		 * will be available at `/foo/actors`.
		 */
		managerBasePath: z.string().optional().default("/"),
		/**
		 * @experimental
		 *
		 * What port to run the manager on.
		 */
		managerPort: z.number().optional().default(6420),

		/** @experimental */
		inspector: InspectorConfigSchema,

		// MARK: Runtime-specific
		serverless: ServerlessConfigSchema.optional().default(() =>
			ServerlessConfigSchema.parse({}),
		),
		runner: RunnerConfigSchema.optional().default(() =>
			RunnerConfigSchema.parse({}),
		),
	})
	.transform((config, ctx) => {
		const isDevEnv = isDev();

		// Parse endpoint string (env var fallback is applied via transform above)
		const parsedEndpoint = config.endpoint
			? tryParseEndpoint(ctx, {
					endpoint: config.endpoint,
					path: ["endpoint"],
					namespace: config.namespace,
					token: config.token,
				})
			: undefined;

		if (parsedEndpoint && config.serveManager) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both endpoint and serveManager",
			});
		}

		// Can't spawn engine AND connect to remote endpoint
		if (config.serverless.spawnEngine && parsedEndpoint) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both spawnEngine and endpoint",
			});
		}

		// configureRunnerPool requires an engine (via endpoint or spawnEngine)
		if (
			config.serverless.configureRunnerPool &&
			!parsedEndpoint &&
			!config.serverless.spawnEngine
		) {
			ctx.addIssue({
				code: "custom",
				message:
					"configureRunnerPool requires either endpoint or spawnEngine",
			});
		}

		// Flatten the endpoint and apply defaults for namespace/token
		// If spawnEngine is enabled, set endpoint to the engine endpoint
		const endpoint = config.serverless.spawnEngine
			? ENGINE_ENDPOINT
			: parsedEndpoint?.endpoint;
		// Namespace priority: parsed from endpoint URL > config value (includes env var) > "default"
		const namespace =
			parsedEndpoint?.namespace ?? config.namespace ?? "default";
		// Token priority: parsed from endpoint URL > config value (includes env var)
		const token = parsedEndpoint?.token ?? config.token;

		// Parse publicEndpoint string (env var fallback is applied via transform in serverless schema)
		const parsedPublicEndpoint = config.serverless.publicEndpoint
			? tryParseEndpoint(ctx, {
					endpoint: config.serverless.publicEndpoint,
					path: ["serverless", "publicEndpoint"],
				})
			: undefined;

		// Validate that publicEndpoint namespace matches backend namespace if specified
		if (
			parsedPublicEndpoint?.namespace &&
			parsedPublicEndpoint.namespace !== namespace
		) {
			ctx.addIssue({
				code: "custom",
				message: `publicEndpoint namespace "${parsedPublicEndpoint.namespace}" must match backend namespace "${namespace}"`,
				path: ["serverless", "publicEndpoint"],
			});
		}

		// Determine serveManager: default to true in dev mode without endpoint, false otherwise
		const serveManager = config.serveManager ?? (isDevEnv && !endpoint);

		// In dev mode, fall back to 127.0.0.1 if serving manager
		const publicEndpoint =
			parsedPublicEndpoint?.endpoint ??
			(isDevEnv && (serveManager || config.serverless.spawnEngine)
				? `http://127.0.0.1:${config.managerPort}`
				: undefined);
		// We extract publicNamespace to validate that it matches the backend
		// namespace (see validation above), not for functional use.
		const publicNamespace = parsedPublicEndpoint?.namespace;
		const publicToken =
			parsedPublicEndpoint?.token ?? config.serverless.publicToken;

		// If endpoint is set or spawning engine, we'll use engine driver - disable manager inspector
		const willUseEngine = !!endpoint || config.serverless.spawnEngine;
		const inspector = willUseEngine
			? {
					...config.inspector,
					enabled: { manager: false, actor: true },
				}
			: config.inspector;

		return {
			...config,
			endpoint,
			namespace,
			token,
			serveManager,
			publicEndpoint,
			publicNamespace,
			publicToken,
			inspector,
			serverless: {
				...config.serverless,
				publicEndpoint,
			},
		};
	});

export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type RegistryConfigInput<A extends RegistryActors> = Omit<
	z.input<typeof RegistryConfigSchema>,
	"use"
> & { use: A };

export function buildActorNames(
	config: RegistryConfig,
): Record<string, { metadata: Record<string, any> }> {
	return Object.fromEntries(
		Object.keys(config.use).map((name) => [name, { metadata: {} }]),
	);
}
