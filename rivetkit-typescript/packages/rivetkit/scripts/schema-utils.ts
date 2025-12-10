import { z } from "zod";

/**
 * Convert Zod schema to JSON Schema with BigInt support.
 *
 * Zod's native z.toJSONSchema() doesn't support BigInt by default. This helper
 * converts z.bigint() to {"type": "integer", "format": "int64"}.
 */
export function toJsonSchema(schema: z.ZodType): any {
	return z.toJSONSchema(schema, {
		unrepresentable: "any",
		override: (ctx) => {
			// Handle BigInt by converting to integer with int64 format
			// Nullable BigInts are handled automatically by Zod's anyOf structure
			if (ctx.zodSchema instanceof z.ZodBigInt) {
				ctx.jsonSchema.type = "integer";
				ctx.jsonSchema.format = "int64";
			}
		},
	});
}
