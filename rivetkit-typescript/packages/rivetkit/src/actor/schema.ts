import type { StandardSchemaV1 } from "@standard-schema/spec";
import { Unsupported } from "./errors";

export interface EventTypeToken<T> {
	readonly _eventType?: T;
}

export interface QueueTypeToken<TMessage, TComplete = never> {
	readonly _queueMessage?: TMessage;
	readonly _queueComplete?: TComplete;
}

/** @deprecated Use `event<T>()`. */
export type Type<T> = EventTypeToken<T>;

export function event<T>(..._args: unknown[]): EventTypeToken<T> {
	return {} as EventTypeToken<T>;
}

export function queue<TMessage, TComplete = never>(
	..._args: unknown[]
): QueueTypeToken<
	TMessage,
	TComplete
> {
	return {} as QueueTypeToken<TMessage, TComplete>;
}

export type PrimitiveSchema = StandardSchemaV1 | EventTypeToken<unknown>;

export interface QueueSchemaDefinition {
	message: PrimitiveSchema;
	complete?: PrimitiveSchema;
}

export type EventSchema = PrimitiveSchema;
export type QueueSchema =
	| PrimitiveSchema
	| QueueSchemaDefinition
	| QueueTypeToken<unknown, unknown>;
export type EventSchemaConfig = Record<string, EventSchema>;
export type QueueSchemaConfig = Record<string, QueueSchema>;
export type AnySchemaConfig = EventSchemaConfig | QueueSchemaConfig;

/** @deprecated Use `EventSchema` or `QueueSchema`. */
export type Schema = QueueSchema;
/** @deprecated Use `EventSchemaConfig` or `QueueSchemaConfig`. */
export type SchemaConfig = QueueSchemaConfig;

export type InferSchema<T> =
	T extends QueueSchemaDefinition
		? InferSchema<T["message"]>
		: T extends QueueTypeToken<infer M, unknown>
		? M
		: T extends StandardSchemaV1<any, infer O>
		? O
		: T extends EventTypeToken<infer R>
			? R
			: never;

export type InferSchemaMap<T extends Record<string, unknown>> = {
	[K in keyof T]: InferSchema<T[K]>;
};

export type InferQueueComplete<T> =
	T extends QueueTypeToken<unknown, infer C>
		? [C] extends [never]
			? never
			: C
		: T extends QueueSchemaDefinition
		? T["complete"] extends PrimitiveSchema
			? InferSchema<T["complete"]>
			: never
		: never;

export type InferQueueCompleteMap<T extends QueueSchemaConfig> = {
	[K in keyof T]: InferQueueComplete<T[K]>;
};

export type InferEventArgs<T> = T extends readonly unknown[]
	? number extends T["length"]
		? [T]
		: T
	: [T];

export type ValidationResult<T> =
	| { success: true; data: T }
	| { success: false; issues: unknown[] };

export function isStandardSchema(value: unknown): value is StandardSchemaV1 {
	return typeof value === "object" && value !== null && "~standard" in value;
}

export function isQueueSchemaDefinition(
	value: unknown,
): value is QueueSchemaDefinition {
	return (
		typeof value === "object" &&
		value !== null &&
		"message" in value &&
		(value as { message?: unknown }).message !== undefined
	);
}

function getValidationSchema(
	schema: QueueSchema | EventSchema | undefined,
): QueueSchema | EventSchema | undefined {
	if (!schema) {
		return undefined;
	}
	if (isQueueSchemaDefinition(schema)) {
		return schema.message;
	}
	return schema;
}

function isPromiseLike(value: unknown): value is PromiseLike<unknown> {
	return (
		typeof value === "object" &&
		value !== null &&
		"then" in value &&
		typeof (value as { then?: unknown }).then === "function"
	);
}

export async function validateSchema<T extends AnySchemaConfig>(
	schemas: T | undefined,
	key: keyof T & string,
	data: unknown,
): Promise<ValidationResult<InferSchemaMap<T>[typeof key]>> {
	const schema = getValidationSchema(schemas?.[key]);

	if (!schema) {
		return { success: true, data: data as InferSchemaMap<T>[typeof key] };
	}

	if (isStandardSchema(schema)) {
		const result = await schema["~standard"].validate(data);
		if (result.issues) {
			return { success: false, issues: [...result.issues] };
		}
		return {
			success: true,
			data: result.value as InferSchemaMap<T>[typeof key],
		};
	}

	return { success: true, data: data as InferSchemaMap<T>[typeof key] };
}

export function validateSchemaSync<T extends AnySchemaConfig>(
	schemas: T | undefined,
	key: keyof T & string,
	data: unknown,
): ValidationResult<InferSchemaMap<T>[typeof key]> {
	const schema = getValidationSchema(schemas?.[key]);

	if (!schema) {
		return { success: true, data: data as InferSchemaMap<T>[typeof key] };
	}

	if (isStandardSchema(schema)) {
		const result = schema["~standard"].validate(data);
		if (isPromiseLike(result)) {
			throw new Unsupported("async schema validation");
		}
		if (result.issues) {
			return { success: false, issues: [...result.issues] };
		}
		return {
			success: true,
			data: result.value as InferSchemaMap<T>[typeof key],
		};
	}

	return { success: true, data: data as InferSchemaMap<T>[typeof key] };
}
