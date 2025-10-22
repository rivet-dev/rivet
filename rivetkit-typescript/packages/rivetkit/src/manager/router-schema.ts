import { z } from "zod";

export const ServerlessStartHeadersSchema = z.object({
	endpoint: z.string({
		required_error: "x-rivet-endpoint header is required",
	}),
	token: z
		.string({ invalid_type_error: "x-rivet-token header must be a string" })
		.optional(),
	totalSlots: z.coerce
		.number({
			invalid_type_error: "x-rivet-total-slots header must be a number",
		})
		.int("x-rivet-total-slots header must be an integer")
		.gte(1, "x-rivet-total-slots header must be positive"),
	runnerName: z.string({
		required_error: "x-rivet-runner-name header is required",
	}),
	namespace: z.string({
		required_error: "x-rivet-namespace-id header is required",
	}),
});
