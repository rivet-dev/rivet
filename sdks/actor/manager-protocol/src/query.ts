import type { ActorTags } from "@rivet-gg/actor-common/utils";
import { z } from "zod";

export const CreateRequestSchema = z.object({
	region: z.string().optional(),
	tags: z.custom<ActorTags>(),
});

export const GetOrCreateRequestSchema = z.object({
	tags: z.custom<ActorTags>(),
	create: CreateRequestSchema.optional(),
});

export const ActorQuerySchema = z.union([
	z.object({
		getForId: z.object({
			actorId: z.string(),
		}),
	}),
	z.object({
		getOrCreateForTags: GetOrCreateRequestSchema,
	}),
	z.object({
		create: CreateRequestSchema,
	}),
]);

export type ActorQuery = z.infer<typeof ActorQuerySchema>;
export type GetOrCreateRequest = z.infer<typeof GetOrCreateRequestSchema>;

// export type CreateRequest = z.infer<typeof CreateRequestSchema>; // Complex type
/**
 * Interface representing a request to create an actor.
 */
export interface CreateRequest {
	/**
	 * The region where the actor should be created.
	 */
	region?: string;

	/**
	 * The tags associated with the actor.
	 */
	tags: { name: string } & { [k: string]: string };
}
