import { getObjectUtils } from "../object";
import type { BaseObjectSchema, ObjectSchema } from "../object/types";
import { getObjectLikeUtils } from "../object-like";
import { getSchemaUtils } from "../schema-utils";
import {
	constructLazyBaseSchema,
	getMemoizedSchema,
	type SchemaGetter,
} from "./lazy";

export function lazyObject<Raw, Parsed>(
	getter: SchemaGetter<ObjectSchema<Raw, Parsed>>,
): ObjectSchema<Raw, Parsed> {
	const baseSchema: BaseObjectSchema<Raw, Parsed> = {
		...constructLazyBaseSchema(getter),
		_getRawProperties: () => getMemoizedSchema(getter)._getRawProperties(),
		_getParsedProperties: () =>
			getMemoizedSchema(getter)._getParsedProperties(),
	};

	return {
		...baseSchema,
		...getSchemaUtils(baseSchema),
		...getObjectLikeUtils(baseSchema),
		...getObjectUtils(baseSchema),
	};
}
