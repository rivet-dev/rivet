import { z } from "zod";

export const ActorTagsSchema = z
	.object({
		name: z.string(),
	})
	.catchall(z.string());

export const BuildTagsSchema = z
	.object({
		name: z.string(),
	})
	.catchall(z.string());

export type ActorTags = z.infer<typeof ActorTagsSchema>;
export type BuildTags = z.infer<typeof BuildTagsSchema>;

export interface RivetEnvironment {
	project?: string;
	environment?: string;
}

export function assertUnreachable(x: never): never {
	throw new Error(`Unreachable case: ${x}`);
}
