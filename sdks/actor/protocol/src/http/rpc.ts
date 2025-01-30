import { z } from "zod";

export const RequestSchema = z.object({
	// Args
	a: z.array(z.unknown()),
});

export const ResponseOkSchema = z.object({
	// Output
	o: z.unknown(),
});

export const ResponseErrSchema = z.object({
	// Code
	c: z.string(),
	// Message
	m: z.string(),
	// Metadata
	md: z.unknown().optional(),
});

export type Request = z.infer<typeof RequestSchema>;
export type ResponseOk = z.infer<typeof ResponseOkSchema>;
export type ResponseErr = z.infer<typeof ResponseErrSchema>;
