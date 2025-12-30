import { ActorDriverBuilder } from "@/actor/driver";
import { z } from "zod";
import {
	getRivetEndpoint,
	getRivetToken,
	getRivetNamespace,
} from "@/utils/env-vars";
import { Logger, LogLevelSchema } from "@/common/log";
import { ManagerDriverBuilder } from "@/manager/driver";
import { InspectorConfigSchema } from "@/inspector/config";

export const DriverConfigSchema = z.object({
	/** Machine-readable name to identify this driver by. */
	name: z.string(),
	displayName: z.string(),
	manager: z.custom<ManagerDriverBuilder>(),
	actor: z.custom<ActorDriverBuilder>(),
	/**
	 * Start actor driver immediately or if this is started separately.
	 *
	 * For example:
	 * - Engine driver needs this to start immediately since this starts the Runner that connects to the engine
	 * - Cloudflare Workers should not start it automatically, since the actor only runs in the DO
	 * */
	autoStartActorDriver: z.boolean(),
});

export type DriverConfig = z.infer<typeof DriverConfigSchema>;

// TODO: Add sane defaults for NODE_ENV=development
export const BaseConfigSchema = z.object({
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
	endpoint: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetEndpoint()),
	token: z
		.string()
		.optional()
		.transform((x) => x ?? getRivetToken()),
	namespace: z.string().default(() => getRivetNamespace() ?? "default"),
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
});
export type BaseConfigInput = z.input<typeof BaseConfigSchema>;
export type BaseConfig = z.infer<typeof BaseConfigSchema>;
