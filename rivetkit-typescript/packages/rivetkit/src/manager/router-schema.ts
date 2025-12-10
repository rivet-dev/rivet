import { z } from "zod";

export const ServerlessStartHeadersSchema = z.object({
	endpoint: z.string({
		error: "x-rivet-endpoint header is required",
	}),
	token: z
		.string({ error: "x-rivet-token header must be a string" })
		.optional(),
	totalSlots: z.coerce
		.number({
			error: "x-rivet-total-slots header must be a number",
		})
		.int({ error: "x-rivet-total-slots header must be an integer" })
		.gte(1, { error: "x-rivet-total-slots header must be positive" }),
	runnerName: z.string({
		error: "x-rivet-runner-name header is required",
	}),
	namespace: z.string({
		error: "x-rivet-namespace-name header is required",
	}),
});
