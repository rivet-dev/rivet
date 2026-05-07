import { z } from "zod";
import { getRunMetadata } from "@/actor/config";
import type {
	AnyActorDefinition,
	BaseActorDefinition,
} from "@/actor/definition";
import {
	KEYS,
	queueMessagesPrefix,
	queueMetadataKey,
	workflowStoragePrefix,
} from "@/actor/keys";
import { ENGINE_ENDPOINT } from "@/common/engine";
import { type Logger, LogLevelSchema } from "@/common/log";
import { VERSION } from "@/utils";
import { tryParseEndpoint } from "@/utils/endpoint-parser";
import {
	getNodeEnv,
	getRivetEndpoint,
	getRivetkitRuntime,
	getRivetNamespace,
	getRivetPool,
	getRivetToken,
	getRivetVersion,
	invalidRivetEnvironmentVariables,
} from "@/utils/env-vars";
import { EnvoyConfigSchema } from "./envoy";
import {
	DEFAULT_SERVERLESS_MAX_START_PAYLOAD_BYTES,
	ServerlessConfigSchema,
} from "./serverless";

export const ActorsSchema = z.record(
	z.string(),
	z.custom<
		BaseActorDefinition<any, any, any, any, any, any, any, any, any>
	>(),
);
export type RegistryActors = z.infer<typeof ActorsSchema>;

export const RuntimeKindSchema = z.enum(["auto", "native", "wasm"]);
export type RuntimeKind = z.infer<typeof RuntimeKindSchema>;
export const RuntimeModeSchema = z.enum(["envoy", "serverless"]);
export type RuntimeMode = z.infer<typeof RuntimeModeSchema>;
export type RuntimeModeSource = "entrypoint" | "default";
export type WasmRuntimeBindings = typeof import("@rivetkit/rivetkit-wasm");
export type WasmRuntimeInitInput = Parameters<
	WasmRuntimeBindings["default"]
>[0];
export const SqliteBackendSchema = z.enum(["local", "remote"]);
export type SqliteBackend = z.infer<typeof SqliteBackendSchema>;

export const TestConfigSchema = z.object({
	enabled: z.boolean().optional().default(false),
	sqliteBackend: SqliteBackendSchema.optional(),
});
export type TestConfig = z.infer<typeof TestConfigSchema>;

export const WasmRuntimeConfigSchema = z.object({
	bindings: z.custom<WasmRuntimeBindings>().optional(),
	initInput: z.custom<WasmRuntimeInitInput>().optional(),
});
export type WasmRuntimeConfig = z.infer<typeof WasmRuntimeConfigSchema>;

export const SqliteConfigSchema = z
	.union([
		SqliteBackendSchema,
		z.object({
			backend: SqliteBackendSchema,
		}),
	])
	.optional()
	.transform((config) => {
		if (config === undefined) return undefined;
		if (typeof config === "string") return { backend: config };
		return config;
	});
export type SqliteConfig = z.infer<typeof SqliteConfigSchema>;

const EngineConfigSchema = z.object({
	endpoint: z.string().optional(),
});

const DevServerlessConfigSchema = z.union([
	z.literal("manual"),
	z.object({
		url: z.string().url(),
		drainTimeout: z.number().int().positive().optional(),
		requestTimeout: z.number().int().positive().optional(),
	}),
]);

const EntrypointConfigSchema = z.discriminatedUnion("kind", [
	z.object({
		kind: z.literal("envoy"),
		envoy: EnvoyConfigSchema.optional().default(() =>
			EnvoyConfigSchema.parse({}),
		),
	}),
	z.object({
		kind: z.literal("serverless"),
		startEngine: z.boolean().optional(),
		devServerless: DevServerlessConfigSchema.optional(),
		serverless: ServerlessConfigSchema.optional().default(() =>
			ServerlessConfigSchema.parse({}),
		),
	}),
	z.object({
		kind: z.literal("listen"),
		startEngine: z.boolean().optional(),
		devServerless: DevServerlessConfigSchema.optional(),
		serverless: ServerlessConfigSchema.optional().default(() =>
			ServerlessConfigSchema.parse({}),
		),
		staticDir: z.string().optional(),
		httpBasePath: z.string().optional().default("/api/rivet"),
		httpPort: z.number().optional().default(6421),
		httpHost: z.string().optional(),
	}),
]);
export type EntrypointConfig = z.infer<typeof EntrypointConfigSchema>;
export type EntrypointConfigInput = z.input<typeof EntrypointConfigSchema>;

function addEnvConfigConflict(
	ctx: z.RefinementCtx,
	envName: string,
	path: (string | number)[],
): void {
	ctx.addIssue({
		code: "custom",
		message: `${envName} and setup(${path.join(".")}) cannot both be set. Use either the environment variable or setup config, not both.`,
		path,
	});
}

function parseEnvNumber(
	ctx: z.RefinementCtx,
	envName: string,
	value: number | undefined,
): number | undefined {
	if (value === undefined) return undefined;
	if (!Number.isInteger(value) || value < 0) {
		ctx.addIssue({
			code: "custom",
			message: `${envName} must be a non-negative integer.`,
		});
		return undefined;
	}
	return value;
}

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
		 * Runtime binding to use for RivetKit core.
		 * */
		runtime: RuntimeKindSchema.optional().transform((val, ctx) => {
			const rawRuntime = val ?? getRivetkitRuntime();
			if (rawRuntime === undefined) {
				return "auto";
			}

			const parsed = RuntimeKindSchema.safeParse(rawRuntime);
			if (!parsed.success) {
				ctx.addIssue({
					code: "custom",
					message:
						"RIVETKIT_RUNTIME must be one of auto, native, or wasm",
				});
				return "auto";
			}

			return parsed.data;
		}),

		/**
		 * @experimental
		 *
		 * WebAssembly runtime configuration.
		 * */
		wasm: WasmRuntimeConfigSchema.optional().default(() => ({})),

		/**
		 * @experimental
		 *
		 * SQLite backend selection.
		 * */
		sqlite: SqliteConfigSchema,

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

		// MARK: Runtime Mode
		entrypoint: EntrypointConfigSchema.optional(),
		mode: z.unknown().optional(),
		pool: z.string().optional(),
		version: z.number().int().nonnegative().optional(),
		devServerless: z.unknown().optional(),

		// MARK: Engine Configuration
		/**
		 * Endpoint to connect to for Rivet Engine.
		 *
		 * Supports URL auth syntax for namespace and token:
		 * - `https://namespace:token@api.rivet.dev`
		 * - `https://namespace@api.rivet.dev`
		 *
		 * Can also be set via RIVET_ENDPOINT environment variables.
		 */
		engine: EngineConfigSchema.optional(),
		/**
		 * @deprecated Use engine.endpoint or RIVET_ENDPOINT.
		 * Kept as an internal escape hatch while framework fixtures migrate.
		 */
		endpoint: z.string().optional(),
		token: z.string().optional(),
		namespace: z.string().optional(),
		headers: z.record(z.string(), z.string()).optional().default({}),

		// MARK: Client
		// TODO:
		// client: ClientConfigSchema.optional(),

		// MARK: Local HTTP
		/**
		 * Directory to serve static files from.
		 *
		 * When set, the local RivetKit server will serve static files from this
		 * directory. This is used by `registry.listen({ static })` to serve a frontend
		 * alongside the actor API.
		 */
		staticDir: z.unknown().optional(),
		/**
		 * @experimental
		 *
		 * Base path for the local RivetKit API. This is used to prefix all routes.
		 * For example, if the base path is `/foo`, then the route `/actors`
		 * will be available at `/foo/actors`.
		 */
		httpBasePath: z.unknown().optional(),
		/**
		 * @experimental
		 *
		 * What port to run the local HTTP server on.
		 */
		httpPort: z.unknown().optional(),
		/**
		 * @experimental
		 *
		 * What host to bind the local HTTP server to.
		 */
		httpHost: z.unknown().optional(),

		// MARK: Engine
		/**
		 * @experimental
		 *
		 * Starts the full Rust engine process locally.
		 */
		startEngine: z.unknown().optional(),
		/** @experimental */
		engineVersion: z.string().optional().default(() => VERSION),
		configurePool: z.unknown().optional(),

		// MARK: Runtime-specific
		serverless: z.unknown().optional(),
		envoy: z.unknown().optional(),

		// MARK: Shutdown
		/**
		 * Graceful shutdown configuration for SIGINT/SIGTERM.
		 *
		 * When a persistent envoy is running (Mode A, started via `registry.start()`),
		 * rivetkit installs Node SIGINT/SIGTERM handlers that call into core's
		 * `shutdown()` and wait up to `gracePeriodMs` for the envoy to drain
		 * before re-raising the signal to let Node exit via its default path.
		 *
		 * Handlers are NOT installed when `handler(request)` is used alone
		 * (Mode B / serverless): platform runtimes (Cloudflare Workers, Vercel,
		 * Deno Deploy) own their own signal policy there, and `process.on` may
		 * not exist.
		 */
		shutdown: z
			.object({
				/**
				 * Wait this many milliseconds for the serve promise to resolve
				 * after calling `CoreRegistry::shutdown()`. Defaults to 30s,
				 * matching Kubernetes `terminationGracePeriodSeconds`.
				 *
				 * Must be >= rivetkit-core's drain timeout (20s) + margin.
				 */
				gracePeriodMs: z
					.number()
					.int()
					.min(1_000)
					.optional()
					.default(30_000),
				/**
				 * If true, rivetkit will not install SIGINT/SIGTERM handlers.
				 * Use when the host application owns signal policy and will
				 * call `nativeRegistry.shutdown()` itself.
				 */
				disableSignalHandlers: z.boolean().optional().default(false),
			})
			.optional()
			.default(() => ({
				gracePeriodMs: 30_000,
				disableSignalHandlers: false,
			})),
	})
	.transform((config, ctx) => {
		for (const invalid of invalidRivetEnvironmentVariables()) {
			ctx.addIssue({
				code: "custom",
				message: invalid.message,
			});
		}
		const removedSetupFields: Array<[string, string]> = [
			["mode", "mode has been removed. Call registry.start() or registry.fetchHandler() instead."],
			["startEngine", "startEngine has been removed from setup(). Use dev.startEngine on registry.listen() or registry.fetchHandler()."],
			["staticDir", "staticDir has been removed from setup(). Use registry.listen({ static }) instead."],
			["httpBasePath", "httpBasePath has been removed from setup(). Use the entrypoint path option instead."],
			["httpPort", "httpPort has been removed from setup(). Use registry.listen({ port }) instead."],
			["httpHost", "httpHost has been removed from setup(). Use registry.listen({ host }) instead."],
			["devServerless", "devServerless has been removed from setup(). Use dev on registry.listen() or registry.fetchHandler()."],
			["serverless", "serverless has been removed from setup(). Pass serverless options to registry.fetchHandler()."],
			["envoy", "envoy has been removed from setup(). Call registry.start() for envoy mode."],
			["configurePool", "configurePool has been removed. Use dev on registry.listen() or registry.fetchHandler()."],
		];
		for (const [field, message] of removedSetupFields) {
			if ((config as Record<string, unknown>)[field] !== undefined) {
				ctx.addIssue({
					code: "custom",
					message,
					path: [field],
				});
			}
		}
		if (config.engine?.endpoint && config.endpoint) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both engine.endpoint and endpoint",
				path: ["engine", "endpoint"],
			});
		}

		const isProduction = getNodeEnv() === "production";
		const sqliteBackend =
			config.sqlite?.backend ?? config.test?.sqliteBackend;
		const entrypoint = config.entrypoint ?? {
			kind: "envoy" as const,
			envoy: EnvoyConfigSchema.parse({}),
		};

		if (config.runtime === "wasm" && sqliteBackend === "local") {
			ctx.addIssue({
				code: "custom",
				message:
					"WebAssembly runtime cannot use local SQLite. Use remote SQLite instead.",
				path:
					config.sqlite?.backend === "local"
						? ["sqlite"]
						: ["test", "sqliteBackend"],
			});
		}

		const sqlite =
			config.runtime === "wasm" && config.sqlite === undefined
				? { backend: "remote" as const }
				: config.sqlite;

		const modeSource: RuntimeModeSource =
			config.entrypoint !== undefined ? "entrypoint" : "default";
		const mode: RuntimeMode =
			entrypoint.kind === "envoy" ? "envoy" : "serverless";

		const envEndpoint = getRivetEndpoint();
		const configEndpoint = config.engine?.endpoint ?? config.endpoint;
		if (envEndpoint !== undefined && configEndpoint !== undefined) {
			addEnvConfigConflict(
				ctx,
				"RIVET_ENDPOINT",
				config.engine?.endpoint ? ["engine", "endpoint"] : ["endpoint"],
			);
		}
		const rawEndpoint = configEndpoint ?? envEndpoint;
		const envToken = getRivetToken();
		if (envToken !== undefined && config.token !== undefined) {
			addEnvConfigConflict(ctx, "RIVET_TOKEN", ["token"]);
		}
		const rawToken = config.token ?? envToken;
		const envNamespace = getRivetNamespace();
		if (envNamespace !== undefined && config.namespace !== undefined) {
			addEnvConfigConflict(ctx, "RIVET_NAMESPACE", ["namespace"]);
		}
		const rawNamespace = config.namespace ?? envNamespace;
		const envPool = getRivetPool();
		if (envPool !== undefined && config.pool !== undefined) {
			addEnvConfigConflict(ctx, "RIVET_POOL", ["pool"]);
		}
		const pool = config.pool ?? envPool ?? "default";
		const envVersion = parseEnvNumber(ctx, "RIVET_VERSION", getRivetVersion());
		if (envVersion !== undefined && config.version !== undefined) {
			addEnvConfigConflict(ctx, "RIVET_VERSION", ["version"]);
		}
		const version = config.version ?? envVersion ?? (isProduction ? undefined : 1);
		if (version === undefined) {
			ctx.addIssue({
				code: "custom",
				message:
					"version or RIVET_VERSION is required when NODE_ENV is production. See https://rivet.dev/docs/actors/versions",
				path: ["version"],
			});
		}
		const entrypointEnvoy =
			entrypoint.kind === "envoy" ? entrypoint.envoy : EnvoyConfigSchema.parse({});
		const envoy = entrypointEnvoy;

		const shouldManageLocalEngine =
			entrypoint.kind === "envoy"
				? !isProduction && rawEndpoint === undefined
				: !isProduction && entrypoint.startEngine === true;
		const devServerless =
			entrypoint.kind === "serverless" || entrypoint.kind === "listen"
				? entrypoint.devServerless
				: undefined;
		const serverless =
			entrypoint.kind === "serverless" || entrypoint.kind === "listen"
				? entrypoint.serverless
				: ServerlessConfigSchema.parse({});
		const staticDir = entrypoint.kind === "listen" ? entrypoint.staticDir : undefined;
		const httpBasePath =
			entrypoint.kind === "listen" ? entrypoint.httpBasePath : "/";
		const httpPort = entrypoint.kind === "listen" ? entrypoint.httpPort : 6421;
		const httpHost = entrypoint.kind === "listen" ? entrypoint.httpHost : undefined;

		// Can't start a local engine and connect to a remote endpoint.
		if (shouldManageLocalEngine && rawEndpoint !== undefined) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both startEngine and endpoint",
			});
		}

		if (mode === "envoy" && isProduction && rawEndpoint === undefined) {
			ctx.addIssue({
				code: "custom",
				message:
					"mode: \"envoy\" requires RIVET_ENDPOINT or engine.endpoint outside local development.",
				path: ["engine", "endpoint"],
			});
		}

		// Parse endpoint string after env/config ambiguity checks.
		const parsedEndpoint = rawEndpoint
			? tryParseEndpoint(ctx, {
					endpoint: rawEndpoint,
					path: config.engine?.endpoint ? ["engine", "endpoint"] : ["endpoint"],
					namespace: rawNamespace,
					token: rawToken,
				})
			: undefined;

		const endpoint = shouldManageLocalEngine
			? ENGINE_ENDPOINT
			: (parsedEndpoint?.endpoint ??
				(mode === "serverless" ? ENGINE_ENDPOINT : undefined));
		const validateServerlessEndpoint = Boolean(
			mode === "serverless" && (shouldManageLocalEngine || parsedEndpoint),
		);
		// Namespace priority: parsed from endpoint URL > config value (includes env var) > "default"
		const namespace =
			parsedEndpoint?.namespace ?? rawNamespace ?? "default";
		// Token priority: parsed from endpoint URL > config value (includes env var)
		const token = parsedEndpoint?.token ?? rawToken;

		// Parse publicEndpoint string (env var fallback is applied via transform in serverless schema)
		const parsedPublicEndpoint = serverless.publicEndpoint
			? tryParseEndpoint(ctx, {
					endpoint: serverless.publicEndpoint,
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
			(shouldManageLocalEngine ? ENGINE_ENDPOINT : undefined);
		// We extract publicNamespace to validate that it matches the backend
		// namespace (see validation above), not for functional use.
		const publicNamespace = parsedPublicEndpoint?.namespace;
		const publicToken =
			parsedPublicEndpoint?.token ?? serverless.publicToken;

		// If endpoint is set or starting the engine, we'll use the engine driver.
		return {
			...config,
			sqlite,
			mode,
			modeSource,
			pool,
			version: version ?? 1,
			startEngine: shouldManageLocalEngine,
			devServerless: isProduction ? undefined : devServerless,
			envoy,
			endpoint,
			namespace,
			token,
			publicEndpoint,
			publicNamespace,
			publicToken,
			validateServerlessEndpoint,
			staticDir,
			httpBasePath,
			httpPort,
			httpHost,
			serverless: {
				...serverless,
				publicEndpoint,
			},
		};
	});

export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type RegistryConfigInput<A extends RegistryActors> = Omit<
	z.input<typeof RegistryConfigSchema>,
	| "use"
	| "entrypoint"
	| "mode"
	| "startEngine"
	| "staticDir"
	| "httpBasePath"
	| "httpPort"
	| "httpHost"
	| "devServerless"
	| "serverless"
	| "envoy"
	| "configurePool"
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
					Array.from(KEYS.LAST_PUSHED_ALARM),
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
					{
						prefix: Array.from(queueMessagesPrefix()),
						maxBytes: 65_536,
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

export const DocDevServerlessConfigSchema = z
	.union([
		z.literal("manual"),
		z.object({
			url: z.string().describe("Local serverless endpoint URL."),
		}),
	])
	.describe("Local serverless development configuration.");

export const DocServerlessConfigSchema = z
	.object({
		basePath: z
			.string()
			.optional()
			.describe(
				"Base path for serverless API routes. Default: '/api/rivet'",
			),
		maxStartPayloadBytes: z
			.number()
			.optional()
			.describe(
				`Maximum POST /start body size in bytes. Default: ${DEFAULT_SERVERLESS_MAX_START_PAYLOAD_BYTES}`,
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
	.object({})
	.describe("Configuration for envoy mode.");

export const DocSqliteConfigSchema = z
	.object({
		backend: SqliteBackendSchema.optional().describe(
			"SQLite backend to use. Native defaults to local. Wasm defaults to remote and cannot use local.",
		),
	})
	.optional()
	.describe("SQLite runtime configuration.");

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
		pool: z
			.string()
			.optional()
			.describe("Pool name. Defaults to 'default'. Can also be set via RIVET_POOL."),
		version: z
			.number()
			.optional()
			.describe("Runtime version. Can also be set via RIVET_VERSION."),
		sqlite: DocSqliteConfigSchema,
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
				"Deprecated. Use engine.endpoint or RIVET_ENDPOINT.",
			),
		engine: z
			.object({
				endpoint: z
					.string()
					.optional()
					.describe("Advanced engine endpoint override. Prefer RIVET_ENDPOINT."),
			})
			.optional(),
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
		engineVersion: z
			.string()
			.optional()
			.describe(
				"Version of the local engine package to use. Defaults to the current RivetKit version.",
			),
	})
	.describe("RivetKit registry configuration.");
