import { z } from "zod";

export const ChangelogItem = z.object({
	published: z.string(),
	images: z.array(z.object({ url: z.string() })),
	title: z.string(),
	description: z.string(),
	slug: z.string(),
	authors: z.array(
		z.object({
			url: z.string(),
			name: z.string(),
			role: z.string(),
			avatar: z.object({ url: z.string() }),
		}),
	),
});
export const Changelog = z.array(ChangelogItem);

export type Changelog = z.infer<typeof Changelog>;
export type ChangelogItem = z.infer<typeof ChangelogItem>;
