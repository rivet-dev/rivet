import type { Logger } from "pino";
import { z } from "zod";
import type { ActorDriverBuilder } from "@/actor/driver";
import { LogLevelSchema } from "@/common/log";
import { EngingConfigSchema as EngineConfigSchema } from "@/drivers/engine/config";
import { InspectorConfigSchema } from "@/inspector/config";
import type { ManagerDriverBuilder } from "@/manager/driver";
import type { GetUpgradeWebSocket } from "@/utils";
import { getEnvUniversal, VERSION } from "@/utils";

export const DriverConfigSchema = z.object({
	/** Machine-readable name to identify this driver by. */
	name: z.string(),
	manager: z.custom<ManagerDriverBuilder>(),
	actor: z.custom<ActorDriverBuilder>(),
});

export type DriverConfig = z.infer<typeof DriverConfigSchema>;

/** Base config used for the actor config across all platforms. */
const RunnerConfigSchemaUnmerged = z
	.object({
		driver: DriverConfigSchema.optional(),

		/** @experimental */
		maxIncomingMessageSize: z.number().optional().default(65_536),

		/** @experimental */
		inspector: InspectorConfigSchema,

		/** @experimental */
		disableDefaultServer: z.boolean().optional().default(false),

		/** @experimental */
		defaultServerPort: z
			.number()
			.default(() => {
				const defaultPort = getEnvUniversal("RIVET_SERVER_PORT");
				if (typeof defaultPort === "undefined") {
					return 6420;
				}
				return parseInt(defaultPort);
			}),

		/** @experimental */
		runEngine: z
			.boolean()
			.optional()
			.default(() => getEnvUniversal("RIVET_RUN_ENGINE") === "1"),

		/** @experimental */
		runEngineVersion: z
			.string()
			.optional()
			.default(
				() => getEnvUniversal("RIVET_RUN_ENGINE_VERSION") ?? VERSION,
			),

		/** @experimental */
		overrideServerAddress: z.string().optional(),

		/** @experimental */
		disableActorDriver: z.boolean().optional().default(false),

		/**
		 * @experimental
		 *
		 * Whether to run runners normally or have them managed
		 * serverlessly (by the Rivet Engine for example).
		 */
		runnerKind: z
			.enum(["serverless", "normal"])
			.optional()
			.default(() =>
				getEnvUniversal("RIVET_RUNNER_KIND") === "serverless"
					? "serverless"
					: "normal",
			),
		totalSlots: z.number().optional(),

		/**
		 * @experimental
		 *
		 * Base path for the router. This is used to prefix all routes.
		 * For example, if the base path is `/api`, then the route `/actors` will be
		 * available at `/api/actors`.
		 */
		basePath: z.string().optional().default("/"),

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

		/**
		 * @experimental
		 *
		 * Automatically configure serverless runners in the engine.
		 * Can only be used when runnerKind is "serverless".
		 * If true, uses default configuration. Can also provide custom configuration.
		 */
		autoConfigureServerless: z
			.union([
				z.boolean(),
				z.object({
					url: z.string().optional(),
					headers: z.record(z.string(), z.string()).optional(),
					maxRunners: z.number().optional(),
					minRunners: z.number().optional(),
					requestLifespan: z.number().optional(),
					runnersMargin: z.number().optional(),
					slotsPerRunner: z.number().optional(),
					metadata: z.record(z.string(), z.unknown()).optional(),
				}),
			])
			.optional(),

		// This is a function to allow for lazy configuration of upgradeWebSocket on the
		// fly. This is required since the dependencies that upgradeWebSocket
		// (specifically Node.js) can sometimes only be specified after the router is
		// created or must be imported async using `await import(...)`
		getUpgradeWebSocket: z.custom<GetUpgradeWebSocket>().optional(),
	})
	.merge(EngineConfigSchema.removeDefault());

const RunnerConfigSchemaBase = RunnerConfigSchemaUnmerged;
export const RunnerConfigSchema = RunnerConfigSchemaBase.default(() =>
	RunnerConfigSchemaBase.parse({}),
);

export type RunnerConfig = z.infer<typeof RunnerConfigSchema>;
export type RunnerConfigInput = z.input<typeof RunnerConfigSchema>;
