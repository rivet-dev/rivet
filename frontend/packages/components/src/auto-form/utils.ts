import type React from "react";
import type { DefaultValues } from "react-hook-form";
import type { z } from "zod";
import type { FieldConfig } from "./types";

// TODO: This should support recursive ZodEffects but TypeScript doesn't allow circular type definitions.
export type ZodObjectOrWrapped =
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	| z.ZodObject<any, any>
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	| z.ZodEffects<z.ZodObject<any, any>>;

/**
 * Beautify a camelCase string.
 * e.g. "myString" -> "My String"
 */
export function beautifyObjectName(string: string) {
	// if numbers only return the string
	let output = string.replace(/([A-Z])/g, " $1");
	output = output.charAt(0).toUpperCase() + output.slice(1);
	return output;
}

/**
 * Get the lowest level Zod type.
 * This will unpack optionals, refinements, etc.
 */
export function getBaseSchema<
	ChildType extends z.ZodAny | z.AnyZodObject = z.ZodAny,
>(schema: ChildType | z.ZodEffects<ChildType>): ChildType | null {
	if (!schema) return null;
	if ("innerType" in schema._def) {
		return getBaseSchema(schema._def.innerType as ChildType);
	}
	if ("schema" in schema._def) {
		return getBaseSchema(schema._def.schema as ChildType);
	}

	return schema as ChildType;
}

/**
 * Get the type name of the lowest level Zod type.
 * This will unpack optionals, refinements, etc.
 */
export function getBaseType(schema: z.ZodAny): string {
	const baseSchema = getBaseSchema(schema);
	return baseSchema ? baseSchema._def.typeName : "";
}

/**
 * Search for a "ZodDefult" in the Zod stack and return its value.
 */
// biome-ignore lint/suspicious/noExplicitAny: FIXME
export function getDefaultValueInZodStack(schema: z.ZodAny): any {
	const typedSchema = schema as unknown as z.ZodDefault<
		z.ZodNumber | z.ZodString
	>;

	if (typedSchema._def.typeName === "ZodDefault") {
		return typedSchema._def.defaultValue();
	}

	if ("innerType" in typedSchema._def) {
		return getDefaultValueInZodStack(
			typedSchema._def.innerType as unknown as z.ZodAny,
		);
	}
	if ("schema" in typedSchema._def) {
		return getDefaultValueInZodStack(
			// biome-ignore lint/suspicious/noExplicitAny: FIXME
			(typedSchema._def as any).schema as z.ZodAny,
		);
	}

	return undefined;
}

/**
 * Get all default values from a Zod schema.
 */
// biome-ignore lint/suspicious/noExplicitAny: FIXME
export function getDefaultValues<Schema extends z.ZodObject<any, any>>(
	schema: Schema,
	fieldConfig?: FieldConfig<z.infer<Schema>>,
) {
	if (!schema) return null;
	const { shape } = schema;
	type DefaultValuesType = DefaultValues<Partial<z.infer<Schema>>>;
	const defaultValues = {} as DefaultValuesType;
	if (!shape) return defaultValues;

	for (const key of Object.keys(shape)) {
		const item = shape[key] as z.ZodAny;

		if (getBaseType(item) === "ZodObject") {
			const defaultItems = getDefaultValues(
				// biome-ignore lint/suspicious/noExplicitAny: FIXME
				getBaseSchema(item) as unknown as z.ZodObject<any, any>,
				fieldConfig?.[key] as FieldConfig<z.infer<Schema>>,
			);

			if (defaultItems !== null) {
				for (const defaultItemKey of Object.keys(defaultItems)) {
					const pathKey =
						`${key}.${defaultItemKey}` as keyof DefaultValuesType;
					defaultValues[pathKey] = defaultItems[defaultItemKey];
				}
			}
		} else {
			let defaultValue = getDefaultValueInZodStack(item);
			if (
				(defaultValue === null || defaultValue === "") &&
				fieldConfig?.[key]?.inputProps
			) {
				defaultValue =
					// biome-ignore lint/suspicious/noExplicitAny: FIXME
					(fieldConfig?.[key]?.inputProps as unknown as any)
						.defaultValue;
			}
			if (defaultValue !== undefined) {
				defaultValues[key as keyof DefaultValuesType] = defaultValue;
			}
		}
	}

	return defaultValues;
}

export function getObjectFormSchema(
	schema: ZodObjectOrWrapped,
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
): z.ZodObject<any, any> {
	if (schema?._def.typeName === "ZodEffects") {
		// biome-ignore lint/suspicious/noExplicitAny: FIXME
		const typedSchema = schema as z.ZodEffects<z.ZodObject<any, any>>;
		return getObjectFormSchema(typedSchema._def.schema);
	}
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	return schema as z.ZodObject<any, any>;
}

/**
 * Convert a Zod schema to HTML input props to give direct feedback to the user.
 * Once submitted, the schema will be validated completely.
 */
export function zodToHtmlInputProps(
	schema:
		| z.ZodNumber
		| z.ZodString
		| z.ZodOptional<z.ZodNumber | z.ZodString>
		// biome-ignore lint/suspicious/noExplicitAny: FIXME
		| any,
): React.InputHTMLAttributes<HTMLInputElement> {
	if (["ZodOptional", "ZodNullable"].includes(schema._def.typeName)) {
		const typedSchema = schema as z.ZodOptional<z.ZodNumber | z.ZodString>;
		return {
			...zodToHtmlInputProps(typedSchema._def.innerType),
			required: false,
		};
	}
	const typedSchema = schema as z.ZodNumber | z.ZodString;

	if (!("checks" in typedSchema._def)) {
		return {
			required: true,
		};
	}

	const { checks } = typedSchema._def;
	const inputProps: React.InputHTMLAttributes<HTMLInputElement> = {
		required: true,
	};
	const type = getBaseType(schema);

	for (const check of checks) {
		if (check.kind === "min") {
			if (type === "ZodString") {
				inputProps.minLength = check.value;
			} else {
				inputProps.min = check.value;
			}
		}
		if (check.kind === "max") {
			if (type === "ZodString") {
				inputProps.maxLength = check.value;
			} else {
				inputProps.max = check.value;
			}
		}
	}

	return inputProps;
}
