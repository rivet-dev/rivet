import { z } from "zod";
import { getRunMetadata } from "@/actor/config";
import type {
	BaseActorDefinition,
	AnyActorDefinition,
} from "@/actor/definition";
import {
	KEYS,
	queueMetadataKey,
	workflowStoragePrefix,
} from "@/actor/keys";
import { type Logger, LogLevelSchema } from "@/common/log";
import { ENGINE_ENDPOINT } from "@/engine-process/constants";
import { InspectorConfigSchema } from "@/inspector/config";
import { DeepReadonly, VERSION } from "@/utils";
import { tryParseEndpoint } from "@/utils/endpoint-parser";
import {
	getRivetEndpoint,
	getRivetEngine,
	getRivetNamespace,
	getRivetRunEngine,
	getRivetRunEngineVersion,
	getRivetToken,
	isDev,
} from "@/utils/env-vars";
import { EnvoyConfigSchema } from "./envoy";
import { ConfigurePoolSchema, ServerlessConfigSchema } from "./serverless";

export const ActorsSchema = z.record(
	z.string(),
	z.custom<
		BaseActorDefinition<any, any, any, any, any, any, any, any, any>
	>(),
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

		// MARK: Local HTTP
		/**
		 * Directory to serve static files from.
		 *
		 * When set, the local RivetKit server will serve static files from this
		 * directory. This is used by `registry.start()` to serve a frontend
		 * alongside the actor API.
		 */
		staticDir: z.string().optional(),
		/**
		 * @experimental
		 *
		 * Base path for the local RivetKit API. This is used to prefix all routes.
		 * For example, if the base path is `/foo`, then the route `/actors`
		 * will be available at `/foo/actors`.
		 */
		httpBasePath: z.string().optional().default("/"),
		/**
		 * @experimental
		 *
		 * What port to run the local HTTP server on.
		 */
		httpPort: z.number().optional().default(6421),
		/**
		 * @experimental
		 *
		 * What host to bind the local HTTP server to.
		 */
		httpHost: z.string().optional(),

		/** @experimental */
		inspector: InspectorConfigSchema,

		// MARK: Engine
		/**
		 * @experimental
		 *
		 * Starts the full Rust engine process locally.
		 */
		startEngine: z.boolean().default(() => getRivetRunEngine()),
		/** @experimental */
		engineVersion: z
			.string()
			.optional()
			.default(() => getRivetRunEngineVersion() ?? VERSION),
		/**
		 * @experimental
		 *
		 * Automatically configure serverless envoys in the engine.
		 */
		configurePool: ConfigurePoolSchema.optional(),

		// MARK: Runtime-specific
		serverless: ServerlessConfigSchema.optional().default(() =>
			ServerlessConfigSchema.parse({}),
		),
		envoy: EnvoyConfigSchema.optional().default(() =>
			EnvoyConfigSchema.parse({}),
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

		// Can't start a local engine and connect to a remote endpoint.
		if (config.startEngine && parsedEndpoint) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both startEngine and endpoint",
			});
		}

		// configurePool requires an engine (via endpoint or startEngine).
		if (config.configurePool && !parsedEndpoint && !config.startEngine) {
			ctx.addIssue({
				code: "custom",
				message:
					"configurePool requires either endpoint or startEngine",
			});
		}

		// Flatten the endpoint and apply defaults for namespace/token
		// If startEngine is enabled, set endpoint to the engine endpoint.
		const endpoint = config.startEngine
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

		// In dev mode, clients connect directly to the local Rivet Engine.
		const publicEndpoint =
			parsedPublicEndpoint?.endpoint ??
			(isDevEnv && config.startEngine ? ENGINE_ENDPOINT : undefined);
		// We extract publicNamespace to validate that it matches the backend
		// namespace (see validation above), not for functional use.
		const publicNamespace = parsedPublicEndpoint?.namespace;
		const publicToken =
			parsedPublicEndpoint?.token ?? config.serverless.publicToken;

		// If endpoint is set or starting the engine, we'll use the engine driver.
		return {
			...config,
			endpoint,
			namespace,
			token,
			publicEndpoint,
			publicNamespace,
			publicToken,
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
): Record<string, { metadata: Record<string, unknown> }> {
	return Object.fromEntries(
		Object.keys(config.use).map((actorName) => {
			const definition = config.use[actorName];
			const options = definition.config.options ?? {};
			const runMeta = getRunMetadata(definition.config.run);
			const metadata: Record<string, unknown> = {};
			// Actor options take precedence over run metadata
			metadata.icon = options.icon ?? runMeta.icon;
			metadata.name = options.name ?? runMeta.name;
			metadata.preload = {
				keys: [
					Array.from(KEYS.PERSIST_DATA),
					Array.from(KEYS.INSPECTOR_TOKEN),
					Array.from(queueMetadataKey()),
				],
				prefixes: [
					{
						prefix: Array.from(workflowStoragePrefix()),
						maxBytes: options.preloadMaxWorkflowBytes ?? 131_072,
						partial: false,
					},
					{
						prefix: Array.from(KEYS.CONN_PREFIX),
						maxBytes: options.preloadMaxConnectionsBytes ?? 65_536,
						partial: false,
					},
				],
			};
			// Remove undefined values
			if (!metadata.icon) delete metadata.icon;
			if (!metadata.name) delete metadata.name;
			return [actorName, { metadata }];
		}),
	);
}

// MARK: Documentation Schemas
// These schemas are JSON-serializable versions used for documentation generation.
// They exclude runtime-only fields (transforms, custom types, Logger instances).

export const DocInspectorConfigSchema = z
	.object({
		enabled: z
			.boolean()
			.optional()
			.describe(
				"Whether to enable the Rivet Inspector. Defaults to true in development mode.",
			),
		token: z
			.string()
			.optional()
			.describe("Token used to access the Inspector."),
		defaultEndpoint: z
			.string()
			.optional()
			.describe(
				"Default RivetKit server endpoint for Rivet Inspector to connect to.",
			),
	})
	.optional()
	.describe("Inspector configuration for debugging and development.");

export const DocConfigurePoolSchema = z
	.object({
		name: z.string().optional().describe("Name of the runner pool."),
		url: z
			.string()
			.describe("URL of the serverless platform to configure runners."),
		headers: z
			.record(z.string(), z.string())
			.optional()
			.describe(
				"Headers to include in requests to the serverless platform.",
			),
		requestLifespan: z
			.number()
			.optional()
			.describe("Maximum lifespan of a request in seconds."),
		metadata: z
			.record(z.string(), z.unknown())
			.optional()
			.describe(
				"Additional metadata to pass to the serverless platform.",
			),
		metadataPollInterval: z
			.number()
			.optional()
			.describe(
				"Interval in milliseconds between metadata polls from the engine. Defaults to 10000 milliseconds (10 seconds).",
			),
		drainOnVersionUpgrade: z
			.boolean()
			.optional()
			.describe(
				"Drain runners when a new version is deployed. Defaults to true.",
			),
	})
	.optional();

export const DocServerlessConfigSchema = z
	.object({
		basePath: z
			.string()
			.optional()
			.describe(
				"Base path for serverless API routes. Default: '/api/rivet'",
			),
		publicEndpoint: z
			.string()
			.optional()
			.describe(
				"The endpoint that clients should connect to. Supports URL auth syntax: https://namespace:token@api.rivet.dev",
			),
		publicToken: z
			.string()
			.optional()
			.describe(
				"Token that clients should use when connecting via the public endpoint.",
			),
	})
	.describe("Configuration for serverless deployment mode.");

export const DocEnvoyConfigSchema = z
	.object({
		totalSlots: z
			.number()
			.optional()
			.describe("Total number of actor slots available. Default: 100000"),
		poolName: z
			.string()
			.optional()
			.describe("Name of this envoy pool. Default: 'default'"),
		envoyKey: z
			.string()
			.optional()
			.describe("Deprecated. Authentication key for the envoy."),
		version: z
			.number()
			.optional()
			.describe("Version number of this envoy. Default: 1"),
	})
	.describe("Configuration for envoy mode.");

export const DocRegistryConfigSchema = z
	.object({
		use: z
			.record(z.string(), z.unknown())
			.describe(
				"Actor definitions. Keys are actor names, values are actor definitions.",
			),
		maxIncomingMessageSize: z
			.number()
			.optional()
			.describe(
				"Maximum size of incoming WebSocket messages in bytes. Default: 65536",
			),
		maxOutgoingMessageSize: z
			.number()
			.optional()
			.describe(
				"Maximum size of outgoing WebSocket messages in bytes. Default: 1048576",
			),
		noWelcome: z
			.boolean()
			.optional()
			.describe("Disable the welcome message on startup. Default: false"),
		logging: z
			.object({
				level: LogLevelSchema.optional().describe(
					"Log level for RivetKit. Default: 'warn'",
				),
			})
			.optional()
			.describe("Logging configuration."),
		endpoint: z
			.string()
			.optional()
			.describe(
				"Endpoint URL to connect to Rivet Engine. Supports URL auth syntax: https://namespace:token@api.rivet.dev. Can also be set via RIVET_ENDPOINT environment variable.",
			),
		token: z
			.string()
			.optional()
			.describe(
				"Authentication token for Rivet Engine. Can also be set via RIVET_TOKEN environment variable.",
			),
		namespace: z
			.string()
			.optional()
			.describe(
				"Namespace to use. Default: 'default'. Can also be set via RIVET_NAMESPACE environment variable.",
			),
		headers: z
			.record(z.string(), z.string())
			.optional()
			.describe(
				"Additional headers to include in requests to Rivet Engine.",
			),
		staticDir: z
			.string()
			.optional()
			.describe(
				"Directory to serve static files from. When set, registry.start() serves static files alongside the actor API.",
			),
		httpBasePath: z
			.string()
			.optional()
			.describe("Base path for the local RivetKit API. Default: '/'"),
		httpPort: z
			.number()
			.optional()
			.describe("Port to run the local HTTP server on. Default: 6421"),
		httpHost: z
			.string()
			.optional()
			.describe("Host to bind the local HTTP server to."),
		inspector: DocInspectorConfigSchema,
		startEngine: z
			.boolean()
			.optional()
			.describe(
				"Starts the full Rust engine process locally. Default: false",
			),
		engineVersion: z
			.string()
			.optional()
			.describe(
				"Version of the local engine package to use. Defaults to the current RivetKit version.",
			),
		configurePool: DocConfigurePoolSchema.describe(
			"Automatically configure serverless runners in the engine.",
		),
		serverless: DocServerlessConfigSchema.optional(),
		envoy: DocEnvoyConfigSchema.optional(),
	})
	.describe("RivetKit registry configuration.");
