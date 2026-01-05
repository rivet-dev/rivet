import invariant from "invariant";
import { z } from "zod";
import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";
import { resolveEndpoint } from "@/client/config";
import { type Logger, LogLevelSchema } from "@/common/log";
import { InspectorConfigSchema } from "@/inspector/config";
import {
	EndpointSchema,
	zodCheckDuplicateCredentials,
} from "@/utils/endpoint-parser";
import { getRivetNamespace, getRivetToken, isDev } from "@/utils/env-vars";
import { type DriverConfig, DriverConfigSchema } from "./driver";
import { RunnerConfigSchema } from "./runner";
import { ServerlessConfigSchema } from "./serverless";

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
		endpoint: EndpointSchema.optional(),
		token: z.string().optional(),
		namespace: z.string().optional(),
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
		const resolvedEndpoint = resolveEndpoint(config.endpoint);

		// Validate duplicate credentials
		if (resolvedEndpoint) {
			zodCheckDuplicateCredentials(resolvedEndpoint, config, ctx);
		}

		if (resolvedEndpoint && config.serveManager) {
			ctx.addIssue({
				code: "custom",
				message: "cannot specify both endpoint and serveManager",
			});
		}

		if (config.serverless) {
			// Can't spawn engine AND connect to remote endpoint
			if (config.serverless.spawnEngine && resolvedEndpoint) {
				ctx.addIssue({
					code: "custom",
					message: "cannot specify both spawnEngine and endpoint",
				});
			}

			// configureRunnerPool requires an engine (via endpoint or spawnEngine)
			if (
				config.serverless.configureRunnerPool &&
				!resolvedEndpoint &&
				!config.serverless.spawnEngine
			) {
				ctx.addIssue({
					code: "custom",
					message:
						"configureRunnerPool requires either endpoint or spawnEngine",
				});
			}

			// advertiseEndpoint required in production without endpoint
			if (
				!isDevEnv &&
				!resolvedEndpoint &&
				!config.serverless.advertiseEndpoint
			) {
				ctx.addIssue({
					code: "custom",
					message:
						"advertiseEndpoint is required in production mode without endpoint",
					path: ["advertiseEndpoint"],
				});
			}
		}

		// Flatten the endpoint and apply defaults for namespace/token
		const endpoint = resolvedEndpoint?.endpoint;
		const namespace =
			resolvedEndpoint?.namespace ??
			config.namespace ??
			getRivetNamespace() ??
			"default";
		const token =
			resolvedEndpoint?.token ?? config.token ?? getRivetToken();

		if (config.serverless) {
			let serveManager: boolean;
			let advertiseEndpoint: string;

			if (endpoint) {
				// Remote endpoint provided:
				// - Do not start manager server
				// - Redirect clients to remote endpoint
				serveManager = config.serveManager ?? false;
				advertiseEndpoint =
					config.serverless.advertiseEndpoint ?? endpoint;
			} else if (isDevEnv) {
				// Development mode, no endpoint:
				// - Start manager server
				// - Redirect clients to local server
				serveManager = config.serveManager ?? true;
				advertiseEndpoint =
					config.serverless.advertiseEndpoint ??
					`http://localhost:${config.managerPort}`;
			} else {
				// Production mode, no endpoint:
				// - Do not start manager server
				// - Use file system driver
				serveManager = config.serveManager ?? false;
				invariant(
					config.serverless.advertiseEndpoint,
					"advertiseEndpoint is required in production mode without endpoint",
				);
				advertiseEndpoint = config.serverless.advertiseEndpoint;
			}

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
				advertiseEndpoint,
				inspector,
				serverless: {
					...config.serverless,
					advertiseEndpoint,
				},
			};
		} else {
			// Runner logic:
			// - If endpoint provided: do not start manager server
			// - If dev mode without endpoint: start manager server
			// - If prod mode without endpoint: do not start manager server
			let serveManager: boolean;
			if (endpoint) {
				serveManager = config.serveManager ?? false;
			} else if (isDevEnv) {
				serveManager = config.serveManager ?? true;
			} else {
				serveManager = config.serveManager ?? false;
			}

			// If endpoint is set, we'll use engine driver - disable manager inspector
			const willUseEngine = !!endpoint;
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
				inspector,
			};
		}
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
