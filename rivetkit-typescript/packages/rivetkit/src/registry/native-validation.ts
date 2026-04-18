import { RivetError } from "@/actor/errors";
import {
	isEventSchemaDefinition,
	isQueueSchemaDefinition,
	isStandardSchema,
	validateSchemaSync,
	type EventSchemaConfig,
	type PrimitiveSchema,
	type QueueSchemaConfig,
} from "@/actor/schema";

const CONN_PARAMS_KEY = "__conn_params__";

export interface NativeValidationConfig {
	actionInputSchemas?: Record<string, PrimitiveSchema>;
	connParamsSchema?: PrimitiveSchema;
	events?: EventSchemaConfig;
	queues?: QueueSchemaConfig;
}

export function validateActionArgs(
	schemas: NativeValidationConfig["actionInputSchemas"],
	name: string,
	args: unknown[],
): unknown[] {
	if (!schemas?.[name]) {
		return args;
	}

	const result = validateSchemaSync(schemas as EventSchemaConfig, name, args);
	if (!result.success) {
		throw validationError(`action \`${name}\` arguments`, result.issues);
	}
	return Array.isArray(result.data) ? result.data : [result.data];
}

export function validateConnParams(
	schema: NativeValidationConfig["connParamsSchema"],
	params: unknown,
): unknown {
	if (!schema) {
		return params;
	}

	const result = validateSchemaSync(
		{ [CONN_PARAMS_KEY]: schema } as EventSchemaConfig,
		CONN_PARAMS_KEY,
		params,
	);
	if (!result.success) {
		throw validationError("connection params", result.issues);
	}
	return result.data;
}

export function validateEventArgs(
	schemas: NativeValidationConfig["events"],
	name: string,
	args: unknown[],
): unknown[] {
	if (!schemas?.[name]) {
		return args;
	}

	const payload = args.length <= 1 ? args[0] : args;
	const result = validateSchemaSync(schemas, name, payload);
	if (!result.success) {
		throw validationError(`event \`${name}\` payload`, result.issues);
	}
	return args.length <= 1
		? [result.data]
		: Array.isArray(result.data)
			? result.data
			: [result.data];
}

export function validateQueueBody(
	schemas: NativeValidationConfig["queues"],
	name: string,
	body: unknown,
): unknown {
	if (!schemas?.[name]) {
		return body;
	}

	const result = validateSchemaSync(schemas, name, body);
	if (!result.success) {
		throw validationError(`queue \`${name}\` message`, result.issues);
	}
	return result.data;
}

export function validateQueueComplete(
	schemas: NativeValidationConfig["queues"],
	name: string,
	response: unknown,
): unknown {
	const schema = schemas?.[name];
	if (!schema) {
		return response;
	}

	let completeSchema: PrimitiveSchema | undefined;
	if (isQueueSchemaDefinition(schema)) {
		completeSchema = schema.complete;
	} else if (
		!isStandardSchema(schema) &&
		!isEventSchemaDefinition(schema) &&
		typeof schema === "object" &&
		schema !== null &&
		"complete" in schema
	) {
		const candidate = (schema as { complete?: unknown }).complete;
		if (candidate !== undefined) {
			completeSchema = candidate as PrimitiveSchema;
		}
	}

	if (!completeSchema) {
		return response;
	}

	const result = validateSchemaSync(
		{ [name]: completeSchema } as EventSchemaConfig,
		name,
		response,
	);
	if (!result.success) {
		throw validationError(`queue \`${name}\` completion response`, result.issues);
	}
	return result.data;
}

function validationError(target: string, issues: unknown[]): RivetError {
	return new RivetError(
		"actor",
		"validation_error",
		`Invalid ${target}`,
		{
			public: true,
			metadata: { issues },
		},
	);
}
