import { z } from "zod/v4";

export const ServerlessStartHeadersSchema = z.object({
	endpoint: z.string({
		error: "x-rivet-endpoint header is required",
	}),
	token: z
		.string({ error: "x-rivet-token header must be a string" })
		.optional(),
	poolName: z.string({
		error: "x-rivet-pool-name header is required",
	}),
	namespace: z.string({
		error: "x-rivet-namespace-name header is required",
	}),
});
