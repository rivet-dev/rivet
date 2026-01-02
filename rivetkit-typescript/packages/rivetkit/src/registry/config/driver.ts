import { z } from "zod";
import { ActorDriverBuilder } from "@/actor/driver";
import { ManagerDriverBuilder } from "@/manager/driver";

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
