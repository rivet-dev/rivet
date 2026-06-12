import { z } from "zod/v4";

// Maximum size of a key component in bytes
// Set to 128 bytes to allow for separators and escape characters in the full key
// Cloudflare's maximum key size is 512 bytes, so we need to be significantly smaller
export const MAX_ACTOR_KEY_SIZE = 128;

export const ActorKeySchema = z.array(z.string().max(MAX_ACTOR_KEY_SIZE));

export type ActorKey = z.infer<typeof ActorKeySchema>;

/**
 * Crash policy for actor lifecycle management.
 *
 * This schema is only used by the engine driver for actor creation. The manager
 * driver ignores crash policy and passes it through to the engine unchanged.
 */
export const CrashPolicySchema = z.enum(["restart", "sleep", "destroy"]);

export type CrashPolicy = z.infer<typeof CrashPolicySchema>;

export const CreateRequestSchema = z.object({
	name: z.string(),
	key: ActorKeySchema,
	input: z.unknown().optional(),
	region: z.string().optional(),
});

export const GetForKeyRequestSchema = z.object({
	name: z.string(),
	key: ActorKeySchema,
});

export const GetOrCreateRequestSchema = z.object({
	name: z.string(),
	key: ActorKeySchema,
	input: z.unknown().optional(),
	region: z.string().optional(),
});

export const ActorQuerySchema = z.union([
	z.object({
		getForId: z.object({
			name: z.string(),
			actorId: z.string(),
		}),
	}),
	z.object({
		getForKey: GetForKeyRequestSchema,
	}),
	z.object({
		getOrCreateForKey: GetOrCreateRequestSchema,
	}),
	z.object({
		create: CreateRequestSchema,
	}),
]);

export type ActorQuery = z.infer<typeof ActorQuerySchema>;
export type ActorGatewayQuery = Extract<
	ActorQuery,
	{ getForKey: unknown } | { getOrCreateForKey: unknown }
>;
/**
 * Interface representing a request to create a actor.
 */
export type CreateRequest = z.infer<typeof CreateRequestSchema>;
