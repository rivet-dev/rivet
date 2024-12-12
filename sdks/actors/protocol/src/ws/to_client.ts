import { z } from "zod";

export const RpcResponseOkSchema = z.object({
	id: z.string(),
	output: z.unknown(),
});

export const RpcResponseErrorSchema = z.object({
	id: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
});

export const ToClientEventSchema = z.object({
	name: z.string(),
	args: z.array(z.unknown()),
});

export const ToClientErrorSchema = z.object({
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
});

export const ToClientSchema = z.object({
	body: z.union([
		z.object({ rpcResponseOk: RpcResponseOkSchema }),
		z.object({ rpcResponseError: RpcResponseErrorSchema }),
		z.object({ event: ToClientEventSchema }),
		z.object({ error: ToClientErrorSchema }),
	]),
});

export type ToClient = z.infer<typeof ToClientSchema>;
export type RpcResponseOk = z.infer<typeof RpcResponseOkSchema>;
export type RpcResponseError = z.infer<typeof RpcResponseErrorSchema>;
export type ToClientEvent = z.infer<typeof ToClientEventSchema>;
export type ToClientError = z.infer<typeof ToClientErrorSchema>;
