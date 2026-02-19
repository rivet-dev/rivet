import type { StandardSchemaV1 } from "@standard-schema/spec";
import { Unsupported } from "./errors";

export const RAW_MARKER = Symbol.for("rivetkit.raw");

export type Raw<T> = {
	[RAW_MARKER]: true;
	_type: T;
};

export function raw<T>(): Raw<T> {
	return { [RAW_MARKER]: true } as Raw<T>;
}

export type Schema = StandardSchemaV1 | Raw<unknown>;

export type SchemaConfig = Record<string, Schema>;

export type InferSchema<T> =
	T extends StandardSchemaV1<any, infer O>
		? O
		: T extends Raw<infer R>
			? R
			: never;

export type InferSchemaMap<T extends SchemaConfig> = {
	[K in keyof T]: InferSchema<T[K]>;
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

export function isRaw(value: unknown): value is Raw<unknown> {
	return typeof value === "object" && value !== null && RAW_MARKER in value;
}

export async function validateSchema<T extends SchemaConfig>(
	schemas: T | undefined,
	key: keyof T & string,
	data: unknown,
): Promise<ValidationResult<InferSchemaMap<T>[typeof key]>> {
	const schema = schemas?.[key];

	if (!schema || isRaw(schema)) {
		return { success: true, data: data as InferSchemaMap<T>[typeof key] };
	}

	if (isStandardSchema(schema)) {
		const result = await schema["~standard"].validate(data);
		if ("issues" in result) {
			return { success: false, issues: result.issues };
		}
		return {
			success: true,
			data: result.value as InferSchemaMap<T>[typeof key],
		};
	}

	return { success: true, data: data as InferSchemaMap<T>[typeof key] };
}

export function validateSchemaSync<T extends SchemaConfig>(
	schemas: T | undefined,
	key: keyof T & string,
	data: unknown,
): ValidationResult<InferSchemaMap<T>[typeof key]> {
	const schema = schemas?.[key];

	if (!schema || isRaw(schema)) {
		return { success: true, data: data as InferSchemaMap<T>[typeof key] };
	}

	if (isStandardSchema(schema)) {
		const result = schema["~standard"].validate(data);
		if (result && typeof (result as Promise<unknown>).then === "function") {
			throw new Unsupported("async schema validation");
		}
		if ("issues" in result) {
			return { success: false, issues: result.issues };
		}
		return {
			success: true,
			data: result.value as InferSchemaMap<T>[typeof key],
		};
	}

	return { success: true, data: data as InferSchemaMap<T>[typeof key] };
}
