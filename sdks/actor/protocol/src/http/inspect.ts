import { z } from "zod";

export const InspectResponseSchema = z.object({
	rpcs: z.array(z.string()),
	state: z.object({
		enabled: z.boolean(),
		native: z.string(),
	}),
	connections: z.number(),
});

export type InspectResponse = z.infer<typeof InspectResponseSchema>;
