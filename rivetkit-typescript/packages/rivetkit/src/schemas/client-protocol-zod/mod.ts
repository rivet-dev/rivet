import { z } from "zod/v4";

// Helper schemas
const UintSchema = z.bigint();
const OptionalUintSchema = UintSchema.nullable();

// MARK: Message To Client
export const InitSchema = z.object({
	actorId: z.string(),
	connectionId: z.string(),
});
export type Init = z.infer<typeof InitSchema>;

export const ErrorSchema = z.object({
	group: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
	actionId: OptionalUintSchema,
});
export type Error = z.infer<typeof ErrorSchema>;

export const ActionResponseSchema = z.object({
	id: UintSchema,
	output: z.unknown(),
});
export type ActionResponse = z.infer<typeof ActionResponseSchema>;

export const EventSchema = z.object({
	name: z.string(),
	args: z.unknown(),
});
export type Event = z.infer<typeof EventSchema>;

export const ToClientBodySchema = z.discriminatedUnion("tag", [
	z.object({ tag: z.literal("Init"), val: InitSchema }),
	z.object({ tag: z.literal("Error"), val: ErrorSchema }),
	z.object({ tag: z.literal("ActionResponse"), val: ActionResponseSchema }),
	z.object({ tag: z.literal("Event"), val: EventSchema }),
]);
export type ToClientBody = z.infer<typeof ToClientBodySchema>;

export const ToClientSchema = z.object({
	body: ToClientBodySchema,
});
export type ToClient = z.infer<typeof ToClientSchema>;

// MARK: Message To Server
export const ActionRequestSchema = z.object({
	id: UintSchema,
	name: z.string(),
	args: z.unknown(),
});
export type ActionRequest = z.infer<typeof ActionRequestSchema>;

export const SubscriptionRequestSchema = z.object({
	eventName: z.string(),
	subscribe: z.boolean(),
});
export type SubscriptionRequest = z.infer<typeof SubscriptionRequestSchema>;

export const ToServerBodySchema = z.discriminatedUnion("tag", [
	z.object({ tag: z.literal("ActionRequest"), val: ActionRequestSchema }),
	z.object({
		tag: z.literal("SubscriptionRequest"),
		val: SubscriptionRequestSchema,
	}),
]);
export type ToServerBody = z.infer<typeof ToServerBodySchema>;

export const ToServerSchema = z.object({
	body: ToServerBodySchema,
});
export type ToServer = z.infer<typeof ToServerSchema>;

// MARK: HTTP Action
export const HttpActionRequestSchema = z.object({
	args: z.unknown(),
});
export type HttpActionRequest = z.infer<typeof HttpActionRequestSchema>;

export const HttpActionResponseSchema = z.object({
	output: z.unknown(),
});
export type HttpActionResponse = z.infer<typeof HttpActionResponseSchema>;

// MARK: HTTP Queue
export const HttpQueueSendRequestSchema = z.object({
	body: z.unknown(),
	name: z.string().optional(),
	wait: z.boolean().optional(),
	timeout: z.number().optional(),
});
export type HttpQueueSendRequest = z.infer<typeof HttpQueueSendRequestSchema>;

export const HttpQueueSendResponseSchema = z.object({
	status: z.enum(["completed", "timedOut"]),
	response: z.unknown().optional(),
});
export type HttpQueueSendResponse = z.infer<typeof HttpQueueSendResponseSchema>;

// MARK: HTTP Error
export const HttpResponseErrorSchema = z.object({
	group: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
});
export type HttpResponseError = z.infer<typeof HttpResponseErrorSchema>;

// MARK: HTTP Resolve
export const HttpResolveRequestSchema = z.null();
export type HttpResolveRequest = z.infer<typeof HttpResolveRequestSchema>;

export const HttpResolveResponseSchema = z.object({
	actorId: z.string(),
});
export type HttpResolveResponse = z.infer<typeof HttpResolveResponseSchema>;
