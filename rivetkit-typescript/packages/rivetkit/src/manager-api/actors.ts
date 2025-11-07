import { z } from "zod";
import { RivetIdSchema } from "./common";

export const ActorSchema = z.object({
	actor_id: RivetIdSchema,
	name: z.string(),
	key: z.string(),
	namespace_id: RivetIdSchema,
	runner_name_selector: z.string(),
	create_ts: z.number(),
	connectable_ts: z.number().nullable().optional(),
	destroy_ts: z.number().nullable().optional(),
	sleep_ts: z.number().nullable().optional(),
	start_ts: z.number().nullable().optional(),
});
export type Actor = z.infer<typeof ActorSchema>;

export const ActorNameSchema = z.object({
	metadata: z.record(z.string(), z.unknown()),
});
export type ActorName = z.infer<typeof ActorNameSchema>;

// MARK: GET /actors
export const ActorsListResponseSchema = z.object({
	actors: z.array(ActorSchema),
});
export type ActorsListResponse = z.infer<typeof ActorsListResponseSchema>;

// MARK: POST /actors
export const ActorsCreateRequestSchema = z.object({
	datacenter: z.string().optional(),
	name: z.string(),
	runner_name_selector: z.string(),
	crash_policy: z.string(),
	key: z.string().nullable().optional(),
	input: z.string().nullable().optional(),
});
export type ActorsCreateRequest = z.infer<typeof ActorsCreateRequestSchema>;

export const ActorsCreateResponseSchema = z.object({
	actor: ActorSchema,
});
export type ActorsCreateResponse = z.infer<typeof ActorsCreateResponseSchema>;

// MARK: PUT /actors
export const ActorsGetOrCreateRequestSchema = z.object({
	datacenter: z.string().optional(),
	name: z.string(),
	key: z.string(),
	runner_name_selector: z.string(),
	crash_policy: z.string(),
	input: z.string().nullable().optional(),
});
export type ActorsGetOrCreateRequest = z.infer<
	typeof ActorsGetOrCreateRequestSchema
>;

export const ActorsGetOrCreateResponseSchema = z.object({
	actor: ActorSchema,
	created: z.boolean(),
});
export type ActorsGetOrCreateResponse = z.infer<
	typeof ActorsGetOrCreateResponseSchema
>;

// MARK: DELETE /actors/{}
export const ActorsDeleteResponseSchema = z.object({});
export type ActorsDeleteResponse = z.infer<typeof ActorsDeleteResponseSchema>;

// MARK: GET /actors/names
export const ActorsListNamesResponseSchema = z.object({
	names: z.record(z.string(), ActorNameSchema),
});
export type ActorsListNamesResponse = z.infer<
	typeof ActorsListNamesResponseSchema
>;
