import { z } from "zod";

export const OuterbaseCommonResponse = z.object({
	success: z.boolean(),
});

export const OuterbaseError = OuterbaseCommonResponse.extend({
	success: z.literal(false),
	error: z.object({
		code: z.string(),
		title: z.string(),
		description: z.string(),
	}),
});
