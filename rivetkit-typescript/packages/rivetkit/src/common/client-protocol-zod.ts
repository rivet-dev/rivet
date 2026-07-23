import { z } from "zod/v4";

// Helper schemas
const UintSchema = z.bigint();
const OptionalUintSchema = UintSchema.nullable();
const normalizeSafeUint = (value: unknown) => {
	if (
		typeof value === "bigint" &&
		value >= 0n &&
		value <= BigInt(Number.MAX_SAFE_INTEGER)
	) {
		return Number(value);
	}
	return value;
};
const SafeUintSchema = z.preprocess(
	normalizeSafeUint,
	z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
);
const ActorSpecifierSchema = z.object({
	actorId: z.string(),
	generation: z.union([z.number(), z.bigint()]),
	key: z.string().optional(),
});

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
	actor: ActorSpecifierSchema.optional(),
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
	dedupeKey: z.string().optional(),
	delay: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER).optional(),
});
export type HttpQueueSendRequest = z.infer<typeof HttpQueueSendRequestSchema>;

export const HttpQueueSendResponseSchema = z.object({
	receiptId: z.string(),
	deduplicated: z.boolean(),
});
export type HttpQueueSendResponse = z.infer<typeof HttpQueueSendResponseSchema>;

const queueStatusWithAttempts = {
	attempts: SafeUintSchema,
	createdAt: SafeUintSchema,
};
export const HttpQueueStatusResponseSchema = z.discriminatedUnion("state", [
	z.object({ state: z.literal("queued"), ...queueStatusWithAttempts }),
	z.object({
		state: z.literal("delayed"),
		...queueStatusWithAttempts,
		availableAt: SafeUintSchema,
	}),
	z.object({
		state: z.literal("processing"),
		...queueStatusWithAttempts,
		startedAt: SafeUintSchema,
	}),
	z.object({
		state: z.literal("retrying"),
		...queueStatusWithAttempts,
		availableAt: SafeUintSchema,
	}),
	z.object({
		state: z.literal("succeeded"),
		...queueStatusWithAttempts,
		completedAt: SafeUintSchema,
	}),
	z.object({
		state: z.literal("deadLettered"),
		...queueStatusWithAttempts,
		failedAt: SafeUintSchema,
	}),
	z.object({
		state: z.literal("consumed"),
		createdAt: SafeUintSchema,
		consumedAt: SafeUintSchema,
	}),
	z.object({ state: z.literal("unknown") }),
]);
export type HttpQueueStatusResponse = z.infer<
	typeof HttpQueueStatusResponseSchema
>;

// MARK: HTTP Error
export const HttpResponseErrorSchema = z.object({
	group: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
	actor: ActorSpecifierSchema.optional(),
});
export type HttpResponseError = z.infer<typeof HttpResponseErrorSchema>;

// MARK: HTTP Resolve
export const HttpResolveRequestSchema = z.null();
export type HttpResolveRequest = z.infer<typeof HttpResolveRequestSchema>;

export const HttpResolveResponseSchema = z.object({
	actorId: z.string(),
});
export type HttpResolveResponse = z.infer<typeof HttpResolveResponseSchema>;
