// Renders JSON schema (draft 7)

import { Foldable } from "@/components/FoldableSchema";
import { cn } from "@rivet-gg/components";
import type {
	JSONSchema7,
	JSONSchema7Definition,
	JSONSchema7Type,
} from "json-schema";
import type { ReactElement, ReactNode } from "react";

interface JsonSchemaPreviewProps {
	className?: string;
	title?: string;
	schema: JSONSchema7;
	defs?: Record<string, JSONSchema7>;
	parent?: string;
	empty?: ReactNode;
}

export function JsonSchemaPreview({
	className,
	title,
	schema: baseSchema,
	defs: baseDefs,
	parent,
	empty,
}: JsonSchemaPreviewProps) {
	const defs =
		baseDefs ??
		(baseSchema.definitions as Record<string, JSONSchema7>) ??
		{};
	const schema = resolveSchema(baseSchema, defs);

	if (schema.type === "object" || schema.$ref) {
		if (
			schema.type === "object" &&
			(!schema.properties || Object.keys(schema.properties).length === 0)
		) {
			return empty;
		}

		return (
			<ObjectSchema
				className={className}
				schema={schema}
				defs={defs}
				parent={parent}
			/>
		);
	}

	return null;
}

interface ObjectSchemaProps {
	schema: JSONSchema7;
	defs: Record<string, JSONSchema7>;
	parent?: string;
	className?: string;
}

function ObjectSchema({
	schema: baseSchema,
	defs,
	parent,
	className,
}: ObjectSchemaProps) {
	const schema = resolveSchema(baseSchema, defs);

	// Merge properties from oneOf variants into main properties
	const allProperties: Record<string, JSONSchema7> = {
		...(schema.properties as Record<string, JSONSchema7> ?? {}),
	};
	const oneOfKeys: Set<string> = new Set();
	if (schema.oneOf) {
		for (const variant of schema.oneOf as JSONSchema7[]) {
			const resolved = resolveSchema(variant, defs);
			if (resolved.properties) {
				for (const [key, value] of Object.entries(resolved.properties)) {
					allProperties[key] = value as JSONSchema7;
					oneOfKeys.add(key);
				}
			}
		}
	}

	return (
		<ul
			className={cn(
				"space-y-4 pl-0 list-none",
				className,
			)}
		>
			{Object.entries(allProperties).map(([key, property]) => {
				const resolved = resolveSchema(property as JSONSchema7, defs);
				// Properties from oneOf are always optional, others check required array
				const nullable = oneOfKeys.has(key) || !schema.required?.some((r) => r === key);
				const newParent = parent ? `${parent}.${key}` : key;

				return (
					<ObjectSchemaItem key={key}>
						<PropertyLabel
							parent={parent}
							name={key}
							schema={resolved}
							defs={defs}
							nullable={nullable}
						/>
						<Schema
							parent={newParent}
							schema={resolved}
							defs={defs}
						/>
					</ObjectSchemaItem>
				);
			})}
			{schema.anyOf ? (
				<li>
					<p className="mb-2">Any of the following variants:</p>
					<ul className="space-y-4 rounded-md border py-4 pl-0 list-none">
						{schema.anyOf?.map((ref: JSONSchema7, index) => {
							return (
								<ObjectSchemaItem
									key={ref.$ref}
								>
									<p>Variant #{index + 1}</p>
									<Schema
										parent={parent}
										schema={ref as JSONSchema7}
										defs={defs}
									/>
								</ObjectSchemaItem>
							);
						})}
					</ul>
				</li>
			) : null}
		</ul>
	);
}

interface SchemaProps {
	schema: JSONSchema7;
	defs: Record<string, JSONSchema7>;
	parent?: string;
	foldable?: boolean;
}

function Schema({ schema: baseSchema, defs, parent, foldable }: SchemaProps) {
	const isFoldable = foldable ?? true;
	const schema = resolveSchema(baseSchema as JSONSchema7, defs);

	// Enum
	if (schema.oneOf) {
		const common = {
			type: "object",
			properties: schema.properties,
		} as JSONSchema7;

		const inner = (
			<ul className="pl-0 list-none">
				{schema.oneOf.map((item: JSONSchema7, index) => {
					return (
						<li
							// biome-ignore lint/suspicious/noArrayIndexKey: only used for static content
							key={index}
							className="mt-4 pl-4"
						>
							{item.enum ? (
								<TypeLabel
									type={item.enum[0]}
									description={item.description}
								/>
							) : (
								<>
									<TypeLabel type={`Variant #${index + 1}`} />
									<Schema
										parent={parent}
										schema={item}
										defs={defs}
										foldable={false}
									/>
								</>
							)}
						</li>
					);
				})}
				{common.properties ? (
					<li className="mt-4 pl-4">
						<TypeLabel type={"Common (on all variants)"} />
						<Schema
							parent={parent}
							schema={common}
							defs={defs}
							foldable={false}
						/>
					</li>
				) : null}
			</ul>
		);

		return isFoldable ? (
			<Foldable
				title="Show possible variants"
				closeTitle="Hide possible variants"
			>
				{inner}
			</Foldable>
		) : (
			inner
		);
	}

	// String enum
	if (schema.enum) {
		const inner = (
			<ul className="space-y-4 rounded-md pl-0 list-none">
				{schema.enum.map((item, index) => {
					return (
						<li
							// biome-ignore lint/suspicious/noArrayIndexKey: only used for static content
							key={index}
							className="mt-4"
						>
							<TypeLabel type={item} />
						</li>
					);
				})}
			</ul>
		);
		return isFoldable ? (
			<Foldable
				title="Show possible variants"
				closeTitle="Hide possible variants"
			>
				{inner}
			</Foldable>
		) : (
			inner
		);
	}

	// Map
	if (schema.additionalProperties) {
		const newParent = `${parent}[key]`;

		const item = resolveSchema(
			schema.additionalProperties as JSONSchema7,
			defs,
		);
		const isObject = item.type === "object";
		const isEmpty =
			item.type === "object" &&
			(!item.properties || Object.keys(item.properties).length === 0);

		if (isObject && !isEmpty) {
			return isFoldable ? (
				<Foldable>
					<JsonSchemaPreview
						schema={item}
						defs={defs}
						parent={newParent}
					/>
				</Foldable>
			) : (
				<JsonSchemaPreview
					schema={item}
					defs={defs}
					parent={newParent}
				/>
			);
		}
		return null;
	}

	// Object
	if (schema.type === "object") {
		const isEmpty =
			!schema.properties || Object.keys(schema.properties).length === 0;

		if (!isEmpty) {
			return isFoldable ? (
				<Foldable>
					<ObjectSchema schema={schema} defs={defs} parent={parent} />
				</Foldable>
			) : (
				<ObjectSchema schema={schema} defs={defs} parent={parent} />
			);
		}
		return null;
	}

	// Array
	if (schema.type === "array" && schema.items) {
		const newParent = `${parent}[]`;

		const items = resolveSchema(schema.items as JSONSchema7, defs);
		const isObject = items.type === "object";
		const isEmpty =
			items.type === "object" &&
			(!items.properties || Object.keys(items.properties).length === 0);
		const isEnum = items.oneOf !== undefined;

		if (isObject && !isEmpty) {
			return isFoldable ? (
				<Foldable
					title={isEnum ? "Show possible variants" : undefined}
					closeTitle={isEnum ? "Hide possible variants" : undefined}
				>
					<Schema
						schema={items}
						defs={defs}
						parent={newParent}
						foldable={false}
					/>
				</Foldable>
			) : (
				<Schema schema={items} defs={defs} parent={newParent} />
			);
		}
		return null;
	}

	return null;
}

interface ObjectSchemaItemProps {
	className?: string;
	children: ReactElement[];
}

function ObjectSchemaItem({ children, className }: ObjectSchemaItemProps) {
	return (
		<li
			className={cn(
				"min-w-0 overflow-auto whitespace-pre border-b pb-4 last:border-none last:pb-0",
				className,
			)}
		>
			{children}
		</li>
	);
}

interface PropertyLabelProps {
	className?: string;
	parent?: string;
	name: string;
	schema: JSONSchema7;
	defs: Record<string, JSONSchema7>;
	nullable: boolean;
}

export function PropertyLabel({
	className,
	parent,
	name,
	schema,
	defs,
	nullable,
}: PropertyLabelProps) {
	return (
		<>
			<div
				className={cn(
					"scrollbar-hide flex items-center gap-1 overflow-auto",
					className,
				)}
			>
				<code className="text-foreground/90">
					{parent ? <>{parent}.</> : null}
					<span className="font-bold text-foreground">{name}</span>
				</code>
				<div className="text-xs text-muted-foreground">
					{getPropertyTypeLabel(schema, defs, nullable)}
				</div>
			</div>
			<div className="prose text-wrap text-sm text-muted-foreground">
				<p>{schema.description || ""}</p>
			</div>
		</>
	);
}

interface TypeLabelProps {
	type: JSONSchema7Type;
	description?: string;
}

function TypeLabel({ type, description }: TypeLabelProps) {
	return (
		<>
			<div className="scrollbar-hide flex items-center gap-1 overflow-auto">
				<code className="font-bold text-foreground">
					{getTypeLabel(type)}
				</code>
			</div>

			<div className="prose text-wrap text-sm text-muted-foreground">
				<p>{description || ""}</p>
			</div>
		</>
	);
}
function getPropertyTypeLabel(
	schema: JSONSchema7,
	defs: Record<string, JSONSchema7>,
	nullable = false,
) {
	const s: string[] = [];

	if (nullable) {
		s.push("nullable");
	}

	if (schema.oneOf) {
		const type = Array.from(
			new Set(
				schema.oneOf.map((s: JSONSchema7) =>
					getPropertyTypeLabel(s, defs),
				),
			),
		);
		s.push(type.join(", "));
	} else if (schema.type === "string") {
		s.push("string");
	} else if (schema.type === "number") {
		s.push("number");
	} else if (schema.type === "integer") {
		s.push("integer");
	} else if (schema.type === "boolean") {
		s.push("boolean");
	} else if (schema.type === "array") {
		s.push(
			`array of ${getPropertyTypeLabel(resolveSchema(schema.items as JSONSchema7, defs), defs)}s`,
		);
	} else if (schema.type === "object") {
		if (schema.additionalProperties) {
			s.push("map");
		} else if (
			!schema.properties ||
			Object.keys(schema.properties).length === 0
		) {
			s.push("empty object");
		} else {
			s.push("object");
		}
	} else if (schema.type === "null") {
		s.push("null");
	} else {
		s.push("unknown");
	}

	// Location from OpenAPI spec
	if (
		// biome-ignore lint/suspicious/noExplicitAny: FIXME
		(schema as any).in
	) {
		s.push(
			`${
				// biome-ignore lint/suspicious/noExplicitAny: FIXME
				(schema as any).in
			} parameter`,
		);
	}

	return s.join(" ");
}

function getTypeLabel(type: JSONSchema7Type) {
	if (Array.isArray(type)) {
		return `[${type.map(getTypeLabel).join(", ")}]`;
	}
	if (type === null) {
		return "null";
	}

	if (typeof type === "object") {
		return `{ ${Object.entries(type)
			.map(([key, type]) => `${key}: ${getTypeLabel(type)}`)
			.join(", ")} }`;
	}

	return type;
}

function resolveSchema(
	schema: JSONSchema7,
	defs: Record<string, JSONSchema7Definition>,
): JSONSchema7 {
	if (schema.allOf?.length) {
		if (schema.allOf.length === 1) {
			return resolveSchema(schema.allOf[0] as JSONSchema7, defs);
		}

		throw new Error("unsupported");
	}

	// Handle anyOf with $ref and null (nullable types)
	if (schema.anyOf?.length) {
		const refItem = schema.anyOf.find(
			(item: JSONSchema7) => item.$ref,
		) as JSONSchema7 | undefined;
		if (refItem) {
			return resolveSchema(refItem, defs);
		}
	}

	if (schema.$ref) {
		const cleanRef = schema.$ref.replace(
			/#\/(definitions|components\/schemas)\//,
			"",
		);
		return defs[cleanRef] as JSONSchema7;
	}

	return schema;
}
