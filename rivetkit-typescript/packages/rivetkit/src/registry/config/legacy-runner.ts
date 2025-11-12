import type { Logger } from "pino";
import { z } from "zod";
import type { ActorDriverBuilder } from "@/actor/driver";
import { LogLevelSchema } from "@/common/log";
import {
	EngineConfigSchemaBase,
	transformEngineConfig,
} from "@/drivers/engine/config";
import { InspectorConfigSchema } from "@/inspector/config";
import type { ManagerDriverBuilder } from "@/manager/driver";
import type { GetUpgradeWebSocket } from "@/utils";
import { getEnvUniversal, VERSION } from "@/utils";
import {
	getRivetRunEngine,
	getRivetRunEngineVersion,
	getRivetRunnerKind,
	getRivetToken,
} from "@/utils/env-vars";

export const LegacyDriverConfigSchema = z.object({
	/** Machine-readable name to identify this driver by. */
	name: z.string(),
	manager: z.custom<ManagerDriverBuilder>(),
	actor: z.custom<ActorDriverBuilder>(),
});

export type LegacyDriverConfig = z.infer<typeof LegacyDriverConfigSchema>;

/** Base config used for the actor config across all platforms. */
const LegacyRunnerConfigSchemaUnmerged = z
	.object({
		driver: LegacyDriverConfigSchema.optional(),

		/** @experimental */
		maxIncomingMessageSize: z.number().optional().default(65_536),

		/** @experimental */
		maxOutgoingMessageSize: z.number().optional().default(1_048_576),

		/** @experimental */
		inspector: InspectorConfigSchema,

		/** @experimental */
		disableDefaultServer: z.boolean().optional().default(false),

		/** @experimental */
		defaultServerPort: z.number().default(6420),

		/** @experimental */
		runEngine: z
			.boolean()
			.optional()
			.default(() => getRivetRunEngine()),

		/** @experimental */
		runEngineVersion: z
			.string()
			.optional()
			.default(() => getRivetRunEngineVersion() ?? VERSION),

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
				getRivetRunnerKind() === "serverless" ? "serverless" : "normal",
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

		/** @experimental */
		token: z
			.string()
			.optional()
			.transform((v) => v || getRivetToken()),
	})
	.merge(EngineConfigSchemaBase);

const LegacyRunnerConfigSchemaTransformed =
	LegacyRunnerConfigSchemaUnmerged.transform((config, ctx) => ({
		...config,
		...transformEngineConfig(config, ctx),
	}));

export const LegacyRunnerConfigSchema =
	LegacyRunnerConfigSchemaTransformed.default(() =>
		LegacyRunnerConfigSchemaTransformed.parse({}),
	);

export type LegacyRunnerConfig = z.infer<typeof LegacyRunnerConfigSchema>;
export type LegacyRunnerConfigInput = z.input<typeof LegacyRunnerConfigSchema>;
