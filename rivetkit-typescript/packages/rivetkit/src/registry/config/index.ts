import { z } from "zod";
import { getRunMetadata } from "@/actor/config";
import type { BaseActorDefinition, AnyActorDefinition } from "@/actor/definition";
import {
	KEYS,
	queueMetadataKey,
	workflowStoragePrefix,
} from "@/actor/instance/keys";
import { type Logger, LogLevelSchema } from "@/common/log";
import { ENGINE_ENDPOINT } from "@/engine-process/constants";
import { InspectorConfigSchema } from "@/inspector/config";
import { DeepReadonly } from "@/utils";
import { tryParseEndpoint } from "@/utils/endpoint-parser";
import {
	getRivetEndpoint,
	getRivetEngine,
	getRivetNamespace,
	getRivetToken,
	isDev,
} from "@/utils/env-vars";
import { EnvoyConfigSchema } from "./envoy";
import { ConfigurePoolSchema, ServerlessConfigSchema } from "./serverless";

export const ActorsSchema = z.record(
	z.string(),
	z.custom<BaseActorDefinition<any, any, any, any, any, any, any, any, any>>(),
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

		// MARK: HTTP server
		/**
		 * Whether to start the local RivetKit HTTP server.
		 * Auto-determined based on endpoint and NODE_ENV if not specified.
		 */
		serveHttp: z.boolean().optional(),
		/**
		 * Directory to serve static files from.
		 *
		 * When set, the local RivetKit HTTP server will serve static files from this
		 * directory. This is used by `registry.start()` to serve a frontend
		 * alongside the actor API.
		 */
		publicDir: z.string().optional(),
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
		 * Port for the local RivetKit HTTP server. Defaults to 8080 so it never
		 * collides with the local engine (fixed on 6420).
		 */
		httpPort: z.number().optional().default(8080),
		/**
		 * @experimental
		 *
		 * What host to bind the local RivetKit HTTP server to.
		 */
		httpHost: z.string().optional(),

		/** @experimental */
		inspector: InspectorConfigSchema,

		// MARK: Runtime mode
		/**
		 * Deployment mode for this registry. Governs whether the user app
		 * runs an HTTP server (`serverless`) or connects out to the engine
		 * as an envoy (`envoy`).
		 *
		 * When omitted the mode is derived from the environment via the
		 * decision matrix:
		 *
		 *                 | Default | NODE_ENV=prod | RIVET_ENDPOINT≠null | mode=envoy override
		 *   spawn_engine  |   y     | error if no   |         n            |         n
		 *                 |         | RIVET_ENDPOINT|                      |
		 *   mode          | envoy   | serverless    |     serverless       |       envoy
		 *
		 * Mode-specific options (e.g. `configurePool`, `publicEndpoint`)
		 * live inside this block and are type-narrowed per mode.
		 */
		runtime: z
			.discriminatedUnion("mode", [
				z.object({
					mode: z.literal("envoy"),
					/**
					 * When set, `registry.start()` spawns a local engine on the
					 * default port. Defaults are derived from the mode matrix.
					 */
					spawnEngine: z.boolean().optional(),
					engineVersion: z.string().optional(),
					poolName: z.string().optional(),
					envoyKey: z.string().optional(),
					version: z.number().optional(),
				}),
				z.object({
					mode: z.literal("serverless"),
					spawnEngine: z.boolean().optional(),
					engineVersion: z.string().optional(),
					poolName: z.string().optional(),
					envoyKey: z.string().optional(),
					version: z.number().optional(),
					configurePool: ConfigurePoolSchema,
					basePath: z.string().optional(),
					publicEndpoint: z.string().optional(),
					publicToken: z.string().optional(),
				}),
			])
			.optional(),

		// MARK: Runtime-specific
		/** @deprecated Use `runtime` with `mode: "serverless"` instead. */
		serverless: ServerlessConfigSchema.optional().default(() =>
			ServerlessConfigSchema.parse({}),
		),
		/** @deprecated Use `runtime` with `mode: "envoy"` instead. */
		envoy: EnvoyConfigSchema.optional().default(() =>
			EnvoyConfigSchema.parse({}),
		),
	})
	.transform((config, ctx) => {
		const isDevEnv = isDev();
		const isProduction =
			typeof process !== "undefined" &&
			process.env.NODE_ENV === "production";

		// Parse endpoint string (env var fallback is applied via transform above)
		const parsedEndpoint = config.endpoint
			? tryParseEndpoint(ctx, {
				endpoint: config.endpoint,
				path: ["endpoint"],
				namespace: config.namespace,
				token: config.token,
			})
			: undefined;

		// Resolve runtime mode when the user didn't explicitly set
		// `runtime`. Localdev Just-Works as envoy (engine spawns locally);
		// only `NODE_ENV=production` flips the auto-default to serverless.
		// `RIVET_ENDPOINT` alone does NOT force serverless — an envoy-mode
		// app can still connect to a remote engine. Users who want
		// serverless in dev must pass `runtime: { mode: "serverless" }`.
		if (config.runtime === undefined) {
			if (isProduction && !parsedEndpoint) {
				ctx.addIssue({
					code: "custom",
					path: ["runtime"],
					message:
						"rivetkit: NODE_ENV=production requires RIVET_ENDPOINT " +
						"(or an explicit `endpoint` config) to connect to a " +
						"hosted engine. Set the env var, or pass " +
						"`runtime: { mode: \"envoy\" }` to opt out of the " +
						"prod serverless default.",
				});
			}
			config.runtime = isProduction
				? { mode: "serverless", configurePool: undefined }
				: { mode: "envoy" };
		}

		// Normalize spawnEngine: `runtime.spawnEngine` is the canonical
		// location. Fall through to the legacy `serverless.spawnEngine`
		// field for back-compat so existing callers keep working during
		// the migration away from rooting spawn config under `serverless`.
		const spawnEngine =
			config.runtime.spawnEngine ?? config.serverless.spawnEngine;
		if (spawnEngine !== undefined) {
			config.serverless.spawnEngine = spawnEngine;
		}

		if (parsedEndpoint && config.serveHttp) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both endpoint and serveHttp",
			});
		}

		// Can't spawn engine AND connect to remote endpoint
		if (config.serverless.spawnEngine && parsedEndpoint) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both spawnEngine and endpoint",
			});
		}

		// configurePool requires an engine (via endpoint or spawnEngine)
		if (
			config.serverless.configurePool &&
			!parsedEndpoint &&
			!config.serverless.spawnEngine
		) {
			ctx.addIssue({
				code: "custom",
				message:
					"configurePool requires either endpoint or spawnEngine",
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

		// Determine serveHttp: default to true in dev mode without endpoint, false otherwise
		const serveHttp = config.serveHttp ?? (isDevEnv && !endpoint);

		// In dev mode, fall back to 127.0.0.1 if serving the local HTTP server
		const publicEndpoint =
			parsedPublicEndpoint?.endpoint ??
			(isDevEnv && (serveHttp || config.serverless.spawnEngine)
				? `http://127.0.0.1:${config.httpPort}`
				: undefined);
		// We extract publicNamespace to validate that it matches the backend
		// namespace (see validation above), not for functional use.
		const publicNamespace = parsedPublicEndpoint?.namespace;
		const publicToken =
			parsedPublicEndpoint?.token ?? config.serverless.publicToken;

		// If endpoint is set or spawning engine, we'll use engine driver - disable HTTP server inspector
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
			serveHttp,
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

export const DocConfigureRunnerPoolSchema = z
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
		maxRunners: z
			.number()
			.optional()
			.describe("Maximum number of runners in the pool."),
		minRunners: z
			.number()
			.optional()
			.describe("Minimum number of runners to keep warm."),
		requestLifespan: z
			.number()
			.optional()
			.describe("Maximum lifespan of a request in milliseconds."),
		runnersMargin: z
			.number()
			.optional()
			.describe("Buffer margin for scaling runners."),
		slotsPerRunner: z
			.number()
			.optional()
			.describe("Number of actor slots per runner."),
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
	})
	.optional();

export const DocServerlessConfigSchema = z
	.object({
		spawnEngine: z
			.boolean()
			.optional()
			.describe(
				"Downloads and starts the full Rust engine process. Auto-enabled in development mode when no endpoint is provided. Default: false",
			),
		engineVersion: z
			.string()
			.optional()
			.describe(
				"Version of the engine to download. Defaults to the current RivetKit version.",
			),
		configureRunnerPool: DocConfigureRunnerPoolSchema.describe(
			"Automatically configure serverless runners in the engine.",
		),
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
		serveHttp: z
			.boolean()
			.optional()
			.describe(
				"Whether to start the local RivetKit HTTP server. Auto-determined based on endpoint and NODE_ENV if not specified.",
			),
		publicDir: z
			.string()
			.optional()
			.describe(
				"Directory to serve static files from. When set, the local RivetKit HTTP server serves static files alongside the actor API. Used by registry.start().",
			),
		httpBasePath: z
			.string()
			.optional()
			.describe("Base path for the local RivetKit API. Default: '/'"),
		httpPort: z
			.number()
			.optional()
			.describe("Port for the local RivetKit HTTP server. Default: 8080"),
		inspector: DocInspectorConfigSchema,
		serverless: DocServerlessConfigSchema.optional(),
		envoy: DocEnvoyConfigSchema.optional(),
	})
	.describe("RivetKit registry configuration.");
