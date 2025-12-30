import { z } from "zod";
import type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";

export const ActorsSchema = z.record(
	z.string(),
	z.custom<ActorDefinition<any, any, any, any, any, any, any>>(),
);
export type RegistryActors = z.infer<typeof ActorsSchema>;

export const TestConfigSchema = z.object({ enabled: z.boolean() });
export type TestConfig = z.infer<typeof TestConfigSchema>;

/** Base config used for the actor config across all platforms. */
export const RegistryConfigSchema = z.object({
	use: z.record(z.string(), z.custom<AnyActorDefinition>()),

	// TODO: Find a better way of passing around the test config
	/**
	 * Test configuration.
	 *
	 * DO NOT MANUALLY ENABLE. THIS IS USED INTERNALLY.
	 * @internal
	 **/
	test: TestConfigSchema.optional().default({ enabled: false }),
});
export type RegistryConfig = z.infer<typeof RegistryConfigSchema>;
export type RegistryConfigInput<A extends RegistryActors> = Omit<
	z.input<typeof RegistryConfigSchema>,
	"use"
> & { use: A };

export function buildActorNames(
	registryConfig: RegistryConfig,
): Record<string, { metadata: Record<string, any> }> {
	return Object.fromEntries(
		Object.keys(registryConfig.use).map((name) => [name, { metadata: {} }]),
	);
}
