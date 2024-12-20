import { z } from "zod";
import type { ActorTags } from "../common/utils.ts";

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
export type CreateRequest = z.infer<typeof CreateRequestSchema>;
