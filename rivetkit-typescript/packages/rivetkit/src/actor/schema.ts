import type { StandardSchemaV1 } from "@standard-schema/spec";
import { Unsupported } from "./errors";

export type SchemaHookResult = boolean | Promise<boolean>;

type SchemaHook<TContext = any> = (ctx: TContext) => SchemaHookResult;

export interface EventTypeToken<T, TContext = any> {
	readonly _eventType?: T;
	readonly canSubscribe?: SchemaHook<TContext>;
	readonly schema?: PrimitiveSchema;
}

export interface QueueTypeToken<
	TMessage,
	TComplete = never,
	TContext = any,
> {
	readonly _queueMessage?: TMessage;
	readonly _queueComplete?: TComplete;
	readonly canPublish?: SchemaHook<TContext>;
	readonly message?: PrimitiveSchema;
	readonly complete?: PrimitiveSchema;
}

/** @deprecated Use `event<T>()`. */
export type Type<T> = EventTypeToken<T, any>;

interface EventOptions<TContext = any> {
	canSubscribe?: SchemaHook<TContext>;
	schema?: PrimitiveSchema;
}

interface QueueOptions<TContext = any> {
	canPublish?: SchemaHook<TContext>;
	message?: PrimitiveSchema;
	complete?: PrimitiveSchema;
}

export function event<T, TContext = any>(
	options?: EventOptions<TContext>,
): EventTypeToken<T, TContext> {
	return (options ?? {}) as EventTypeToken<T, TContext>;
}

export function queue<TMessage, TComplete = never, TContext = any>(
	options?: QueueOptions<TContext>,
): QueueTypeToken<
	TMessage,
	TComplete,
	TContext
> {
	return (options ?? {}) as QueueTypeToken<TMessage, TComplete, TContext>;
}

export type PrimitiveSchema = StandardSchemaV1 | EventTypeToken<unknown, any>;

export interface EventSchemaDefinition<TContext = any> {
	schema: PrimitiveSchema;
	canSubscribe?: SchemaHook<TContext>;
}

export interface QueueSchemaDefinition<TContext = any> {
	message: PrimitiveSchema;
	complete?: PrimitiveSchema;
	canPublish?: SchemaHook<TContext>;
}

export type EventSchema<TContext = any> =
	| PrimitiveSchema
	| EventSchemaDefinition<TContext>;
export type QueueSchema =
	| PrimitiveSchema
	| QueueSchemaDefinition<any>
	| QueueTypeToken<unknown, unknown, any>;
export type EventSchemaConfig<TContext = any> = Record<
	string,
	EventSchema<TContext>
>;
export type QueueSchemaConfig<TContext = any> = Record<string, QueueSchema>;
export type AnySchemaConfig = EventSchemaConfig | QueueSchemaConfig;

/** @deprecated Use `EventSchema` or `QueueSchema`. */
export type Schema = QueueSchema;
/** @deprecated Use `EventSchemaConfig` or `QueueSchemaConfig`. */
export type SchemaConfig = QueueSchemaConfig;

export type InferSchema<T> =
	T extends QueueSchemaDefinition<any>
			? InferSchema<T["message"]>
			: T extends QueueTypeToken<infer M, unknown, any>
			? M
			: T extends EventSchemaDefinition<any>
			? InferSchema<T["schema"]>
			: T extends StandardSchemaV1<any, infer O>
			? O
			: T extends EventTypeToken<infer R, any>
				? R
				: never;

export type InferSchemaMap<T extends Record<string, unknown>> = {
	[K in keyof T]: InferSchema<T[K]>;
};

export type InferQueueComplete<T> =
	T extends QueueTypeToken<unknown, infer C, any>
			? [C] extends [never]
				? never
				: C
			: T extends QueueSchemaDefinition<any>
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
): value is QueueSchemaDefinition<any> {
	if (isEventSchemaDefinition(value)) {
		return false;
	}
	return (
		typeof value === "object" &&
		value !== null &&
		"message" in value &&
		(value as { message?: unknown }).message !== undefined
	);
}

export function isEventSchemaDefinition(
	value: unknown,
): value is EventSchemaDefinition<any> {
	return (
		typeof value === "object" &&
		value !== null &&
		"schema" in value &&
		(value as { schema?: unknown }).schema !== undefined
	);
}

export function hasSchemaConfigKey<T extends AnySchemaConfig>(
	schemas: T | undefined,
	key: string,
): boolean {
	if (!schemas) {
		return false;
	}
	return Object.prototype.hasOwnProperty.call(schemas, key);
}

export function getEventCanSubscribe<TContext = any>(
	schemas: EventSchemaConfig<TContext> | undefined,
	key: string,
): SchemaHook<TContext> | undefined {
	const schema = schemas?.[key];
	if (!schema || isStandardSchema(schema)) {
		return undefined;
	}

	const maybeCanSubscribe = (schema as { canSubscribe?: unknown })
		.canSubscribe;
	return typeof maybeCanSubscribe === "function"
		? (maybeCanSubscribe as SchemaHook<TContext>)
		: undefined;
}

export function getQueueCanPublish<TContext = any>(
	schemas: QueueSchemaConfig | undefined,
	key: string,
): SchemaHook<TContext> | undefined {
	const schema = schemas?.[key];
	if (!schema || isStandardSchema(schema)) {
		return undefined;
	}

	const maybeCanPublish = (schema as { canPublish?: unknown }).canPublish;
	return typeof maybeCanPublish === "function"
		? (maybeCanPublish as SchemaHook<TContext>)
		: undefined;
}

function getValidationSchema(
	schema: QueueSchema | EventSchema | undefined,
): QueueSchema | EventSchema | undefined {
	if (!schema) {
		return undefined;
	}
	if (isEventSchemaDefinition(schema)) {
		return schema.schema;
	}
	if (isQueueSchemaDefinition(schema)) {
		return schema.message;
	}
	if (
		typeof schema === "object" &&
		schema !== null &&
		"schema" in schema &&
		(schema as { schema?: unknown }).schema !== undefined
	) {
		return (schema as { schema: QueueSchema | EventSchema }).schema;
	}
	if (
		typeof schema === "object" &&
		schema !== null &&
		"message" in schema &&
		(schema as { message?: unknown }).message !== undefined
	) {
		return (schema as { message: QueueSchema | EventSchema }).message;
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
