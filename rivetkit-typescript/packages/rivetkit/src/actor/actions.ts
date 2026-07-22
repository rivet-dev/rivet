import type { PrimitiveSchema } from "./schema";

export type RuntimeActionHandler = (...args: any[]) => any;

export function flattenActionHandlers(
	actions: unknown,
): Record<string, RuntimeActionHandler> {
	const flattened = Object.create(null) as Record<
		string,
		RuntimeActionHandler
	>;
	for (const { name, handler } of collectActionEntries(actions)) {
		flattened[name] = handler;
	}
	return flattened;
}

export function flattenActionInputSchemas(
	actions: unknown,
	schemas: unknown,
): Record<string, PrimitiveSchema> | undefined {
	if (schemas === undefined) return undefined;
	if (!isRecord(schemas)) {
		throw new TypeError("actionInputSchemas must be an object");
	}

	const flattened = Object.create(null) as Record<string, PrimitiveSchema>;
	for (const { name, path } of collectActionEntries(actions)) {
		const nestedSchema = lookupNestedSchema(schemas, path);
		const flatSchema = schemas[name];
		if (
			nestedSchema !== undefined &&
			flatSchema !== undefined &&
			nestedSchema !== flatSchema
		) {
			throw new TypeError(
				`Action input schema \`${name}\` is defined by both a nested path and a dotted key`,
			);
		}

		const schema = nestedSchema ?? flatSchema;
		if (schema !== undefined) {
			flattened[name] = schema as PrimitiveSchema;
		}
	}
	return flattened;
}

interface ActionEntry {
	name: string;
	path: string[];
	handler: RuntimeActionHandler;
}

function collectActionEntries(actions: unknown): ActionEntry[] {
	const entries: ActionEntry[] = [];
	const names = new Set<string>();
	visitActionGroup(actions ?? {}, [], entries, names);
	return entries;
}

function visitActionGroup(
	value: unknown,
	path: string[],
	entries: ActionEntry[],
	names: Set<string>,
) {
	if (!isRecord(value)) {
		throw new TypeError(
			`${formatActionPath(path)} must be an action handler or group`,
		);
	}

	for (const [segment, child] of Object.entries(value)) {
		const childPath = [...path, segment];
		if (typeof child === "function") {
			const name = childPath.join(".");
			if (names.has(name)) {
				throw new TypeError(
					`Multiple action definitions flatten to \`${name}\``,
				);
			}
			names.add(name);
			entries.push({
				name,
				path: childPath,
				handler: child as RuntimeActionHandler,
			});
		} else {
			visitActionGroup(child, childPath, entries, names);
		}
	}
}

function lookupNestedSchema(
	schemas: Record<string, unknown>,
	path: string[],
): unknown {
	let value: unknown = schemas;
	for (const segment of path) {
		if (!isRecord(value) || !Object.hasOwn(value, segment)) {
			return undefined;
		}
		value = value[segment];
	}
	return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) {
		return false;
	}
	const prototype = Object.getPrototypeOf(value);
	return prototype === Object.prototype || prototype === null;
}

function formatActionPath(path: string[]): string {
	return path.length === 0 ? "actions" : `Action \`${path.join(".")}\``;
}
