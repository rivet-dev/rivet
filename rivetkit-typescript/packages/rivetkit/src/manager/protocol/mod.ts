import { z } from "zod";
import { ActorQuerySchema } from "./query";

export * from "./query";

export const ActorsRequestSchema = z.object({
	query: ActorQuerySchema,
});

export const ActorsResponseSchema = z.object({
	actorId: z.string(),
});

//export const RivetConfigResponseSchema = z.object({
//	endpoint: z.string(),
//	project: z.string().optional(),
//	environment: z.string().optional(),
//});

export type ActorsRequest = z.infer<typeof ActorsRequestSchema>;
export type ActorsResponse = z.infer<typeof ActorsResponseSchema>;
//export type RivetConfigResponse = z.infer<typeof RivetConfigResponseSchema>;
