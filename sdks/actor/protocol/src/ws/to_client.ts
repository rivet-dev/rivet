import { z } from "zod";

export const RpcResponseOkSchema = z.object({
	// ID
	i: z.number().int(),
	// Output
	o: z.unknown(),
});

export const RpcResponseErrorSchema = z.object({
	// ID
	i: z.number().int(),
	// Code
	c: z.string(),
	// Message
	m: z.string(),
	// Metadata
	md: z.unknown().optional(),
});

export const ToClientEventSchema = z.object({
	// Name
	n: z.string(),
	// Args
	a: z.array(z.unknown()),
});

export const ToClientErrorSchema = z.object({
	// Code
	c: z.string(),
	// Message
	m: z.string(),
	// Metadata
	md: z.unknown().optional(),
});

export const ToClientSchema = z.object({
	body: z.union([
		z.object({ ro: RpcResponseOkSchema }),
		z.object({ re: RpcResponseErrorSchema }),
		z.object({ ev: ToClientEventSchema }),
		z.object({ er: ToClientErrorSchema }),
	]),
});

export type ToClient = z.infer<typeof ToClientSchema>;
export type RpcResponseOk = z.infer<typeof RpcResponseOkSchema>;
export type RpcResponseError = z.infer<typeof RpcResponseErrorSchema>;
export type ToClientEvent = z.infer<typeof ToClientEventSchema>;
export type ToClientError = z.infer<typeof ToClientErrorSchema>;

// MARK: Internal RPCs

export const InspectRpcResponseSchema = z.object({
	rpcs: z.array(z.string()),
	state: z.object({
		enabled: z.boolean(),
		native: z.string(),
	}),
	connections: z.array(
		z.object({
			id: z.string(),
			state: z.any(),
			subscriptions: z.array(z.string()),
		}),
	),
});

export type InspectRpcResponse = z.infer<typeof InspectRpcResponseSchema>;
