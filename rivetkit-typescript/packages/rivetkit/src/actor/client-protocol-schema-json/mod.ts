import { z } from "zod";

// Helper schemas for ArrayBuffer handling in JSON
const ArrayBufferSchema = z.instanceof(ArrayBuffer);
const OptionalArrayBufferSchema = ArrayBufferSchema.nullable();
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
	metadata: OptionalArrayBufferSchema,
	actionId: OptionalUintSchema,
});
export type Error = z.infer<typeof ErrorSchema>;

export const ActionResponseSchema = z.object({
	id: UintSchema,
	output: ArrayBufferSchema,
});
export type ActionResponse = z.infer<typeof ActionResponseSchema>;

export const EventSchema = z.object({
	name: z.string(),
	args: ArrayBufferSchema,
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
	args: ArrayBufferSchema,
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
	args: ArrayBufferSchema,
});
export type HttpActionRequest = z.infer<typeof HttpActionRequestSchema>;

export const HttpActionResponseSchema = z.object({
	output: ArrayBufferSchema,
});
export type HttpActionResponse = z.infer<typeof HttpActionResponseSchema>;

// MARK: HTTP Error
export const HttpResponseErrorSchema = z.object({
	group: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: OptionalArrayBufferSchema,
});
export type HttpResponseError = z.infer<typeof HttpResponseErrorSchema>;

// MARK: HTTP Resolve
export const HttpResolveRequestSchema = z.null();
export type HttpResolveRequest = z.infer<typeof HttpResolveRequestSchema>;

export const HttpResolveResponseSchema = z.object({
	actorId: z.string(),
});
export type HttpResolveResponse = z.infer<typeof HttpResolveResponseSchema>;
