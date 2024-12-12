import { z } from "zod";

const RpcRequestSchema = z.object({
	id: z.string(),
	name: z.string(),
	args: z.array(z.unknown()),
});

const SubscriptionRequestSchema = z.object({
	eventName: z.string(),
	subscribe: z.boolean(),
});

export const ToServerSchema = z.object({
	body: z.union([
		z.object({ rpcRequest: RpcRequestSchema }),
		z.object({ subscriptionRequest: SubscriptionRequestSchema }),
	]),
});

// Type inference
export type ToServer = z.infer<typeof ToServerSchema>;
export type RpcRequest = z.infer<typeof RpcRequestSchema>;
export type SubscriptionRequest = z.infer<typeof SubscriptionRequestSchema>;
