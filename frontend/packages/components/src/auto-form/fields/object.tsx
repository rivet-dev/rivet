import { useFormContext } from "react-hook-form";
import * as z from "zod";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "../../ui/accordion";
import { FormField } from "../../ui/form";
import { MutedText } from "../../ui/typography";
import { DEFAULT_ZOD_HANDLERS, INPUT_COMPONENTS } from "../config";
import resolveDependencies from "../dependencies";
import type { Dependency, FieldConfig, FieldConfigItem } from "../types";
import {
	beautifyObjectName,
	getBaseSchema,
	getBaseType,
	zodToHtmlInputProps,
} from "../utils";
import AutoFormArray from "./array";

function DefaultParent({ children }: { children: React.ReactNode }) {
	return <>{children}</>;
}

export default function AutoFormObject<
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	SchemaType extends z.ZodObject<any, any>,
>({
	schema,
	fieldConfig,
	path = [],
	dependencies = [],
}: {
	schema: SchemaType | z.ZodEffects<SchemaType>;
	fieldConfig?: FieldConfig<z.infer<SchemaType>>;
	path?: string[];
	dependencies?: Dependency<z.infer<SchemaType>>[];
}) {
	const { watch, control } = useFormContext(); // Use useFormContext to access the watch function

	if (!schema) {
		return null;
	}
	const { shape } = getBaseSchema<SchemaType>(schema) || {};

	if (!shape) {
		return null;
	}

	const handleIfZodNumber = (item: z.ZodAny) => {
		// biome-ignore lint/suspicious/noExplicitAny: FIXME
		const isZodNumber = (item as any)._def.typeName === "ZodNumber";
		const isInnerZodNumber =
			// biome-ignore lint/suspicious/noExplicitAny: FIXME
			(item._def as any).innerType?._def?.typeName === "ZodNumber";

		if (isZodNumber) {
			// biome-ignore lint/suspicious/noExplicitAny: FIXME
			(item as any)._def.coerce = true;
		} else if (isInnerZodNumber) {
			// biome-ignore lint/suspicious/noExplicitAny: FIXME
			(item._def as any).innerType._def.coerce = true;
		}

		return item;
	};

	const shapeKeys = Object.keys(shape);

	return (
		<Accordion type="multiple" className="space-y-5 border-none">
			{shapeKeys.length === 0 && (
				<MutedText>No fields to configure.</MutedText>
			)}
			{shapeKeys.map((name) => {
				let item = shape[name] as z.ZodAny;
				item = handleIfZodNumber(item) as z.ZodAny;
				const zodBaseType = getBaseType(item);
				const itemName =
					item._def.description ?? beautifyObjectName(name);
				const key = [...path, name].join(".");

				const {
					isHidden,
					isDisabled,
					isRequired: isRequiredByDependency,
					overrideOptions,
				} = resolveDependencies(dependencies, name, watch);
				if (isHidden) {
					return null;
				}

				if (zodBaseType === "ZodObject") {
					return (
						<AccordionItem
							value={name}
							key={key}
							className="border-none"
						>
							<AccordionTrigger>{itemName}</AccordionTrigger>
							<AccordionContent className="p-2">
								<AutoFormObject
									schema={
										// biome-ignore lint/suspicious/noExplicitAny: FIXME
										item as unknown as z.ZodObject<any, any>
									}
									fieldConfig={
										(fieldConfig?.[name] ??
											{}) as FieldConfig<
											z.infer<typeof item>
										>
									}
									path={[...path, name]}
								/>
							</AccordionContent>
						</AccordionItem>
					);
				}
				if (zodBaseType === "ZodArray") {
					return (
						<AutoFormArray
							key={key}
							name={name}
							item={
								// biome-ignore lint/suspicious/noExplicitAny: FIXME
								item as unknown as z.ZodArray<any>
							}
							fieldConfig={fieldConfig?.[name] ?? {}}
							path={[...path, name]}
						/>
					);
				}

				const fieldConfigItem: FieldConfigItem =
					fieldConfig?.[name] ?? {};
				const zodInputProps = zodToHtmlInputProps(item);
				const isRequired =
					isRequiredByDependency ||
					zodInputProps.required ||
					fieldConfigItem.inputProps?.required ||
					false;

				if (overrideOptions) {
					item = z.enum(overrideOptions) as unknown as z.ZodAny;
				}

				return (
					<FormField
						control={control}
						name={key}
						key={key}
						render={({ field }) => {
							const inputType =
								fieldConfigItem.fieldType ??
								DEFAULT_ZOD_HANDLERS[zodBaseType] ??
								"fallback";

							const InputComponent =
								typeof inputType === "function"
									? inputType
									: INPUT_COMPONENTS[inputType];

							const ParentElement =
								fieldConfigItem.renderParent ?? DefaultParent;

							const defaultValue =
								fieldConfigItem.inputProps?.defaultValue;
							const value = field.value ?? defaultValue ?? "";

							const fieldProps = {
								...zodToHtmlInputProps(item),
								...field,
								...fieldConfigItem.inputProps,
								disabled:
									fieldConfigItem.inputProps?.disabled ||
									isDisabled,
								ref: undefined,
								value: value,
							};

							if (InputComponent === undefined) {
								return <></>;
							}

							return (
								<ParentElement key={`${key}.parent`}>
									<InputComponent
										zodInputProps={zodInputProps}
										field={field}
										fieldConfigItem={fieldConfigItem}
										label={itemName}
										isRequired={isRequired}
										zodItem={item}
										fieldProps={fieldProps}
										path={[...path, name]}
										className={fieldProps.className}
									/>
								</ParentElement>
							);
						}}
					/>
				);
			})}
		</Accordion>
	);
}
