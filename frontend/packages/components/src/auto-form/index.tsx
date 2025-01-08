"use client";
import React from "react";
import { type DefaultValues, type FormState, useForm } from "react-hook-form";
import type { z } from "zod";
import { Form } from "../ui/form";

import { zodResolver } from "@hookform/resolvers/zod";
import { cn } from "../lib/utils";
import { Button } from "../ui/button";

import AutoFormObject from "./fields/object";
import type { Dependency, FieldConfig } from "./types";
import {
	type ZodObjectOrWrapped,
	getDefaultValues,
	getObjectFormSchema,
} from "./utils";

export { AutoFormObject };

export function AutoFormSubmit({
	children,
	className,
	disabled,
}: {
	children?: React.ReactNode;
	className?: string;
	disabled?: boolean;
}) {
	return (
		<Button type="submit" disabled={disabled} className={className}>
			{children ?? "Submit"}
		</Button>
	);
}

export function AutoForm<SchemaType extends ZodObjectOrWrapped>({
	formSchema,
	values: valuesProp,
	onValuesChange: onValuesChangeProp,
	onParsedValuesChange,
	onSubmit: onSubmitProp,
	fieldConfig,
	children,
	className,
	dependencies,
}: {
	formSchema: SchemaType;
	values?: Partial<z.infer<SchemaType>>;
	onValuesChange?: (values: Partial<z.infer<SchemaType>>) => void;
	onParsedValuesChange?: (values: Partial<z.infer<SchemaType>>) => void;
	onSubmit?: (values: z.infer<SchemaType>) => void;
	fieldConfig?: FieldConfig<z.infer<SchemaType>>;
	children?:
		| React.ReactNode
		| ((formState: FormState<z.infer<SchemaType>>) => React.ReactNode);
	className?: string;
	dependencies?: Dependency<z.infer<SchemaType>>[];
}) {
	const objectFormSchema = getObjectFormSchema(formSchema);
	const defaultValues: DefaultValues<
		z.infer<typeof objectFormSchema>
	> | null = getDefaultValues(objectFormSchema, fieldConfig);

	const form = useForm<z.infer<typeof objectFormSchema>>({
		resolver: zodResolver(formSchema),
		defaultValues: defaultValues ?? undefined,
		values: valuesProp,
	});

	function onSubmit(values: z.infer<typeof formSchema>) {
		const parsedValues = formSchema.safeParse(values);
		if (parsedValues.success) {
			onSubmitProp?.(parsedValues.data);
		}
	}

	const values = form.watch();
	// valuesString is needed because form.watch() returns a new object every time
	const valuesString = JSON.stringify(values);

	// biome-ignore lint/correctness/useExhaustiveDependencies: FIXME
	React.useEffect(() => {
		onValuesChangeProp?.(values);
		const parsedValues = formSchema.safeParse(values);
		if (parsedValues.success) {
			onParsedValuesChange?.(parsedValues.data);
		}
	}, [valuesString]);

	const renderChildren =
		typeof children === "function"
			? children(form.formState as FormState<z.infer<SchemaType>>)
			: children;

	return (
		<div className="w-full">
			<Form {...form}>
				<form
					onSubmit={(e) => {
						form.handleSubmit(onSubmit)(e);
					}}
					className={cn("space-y-5", className)}
				>
					<AutoFormObject
						schema={objectFormSchema}
						dependencies={dependencies}
						fieldConfig={fieldConfig}
					/>

					{renderChildren}
				</form>
			</Form>
		</div>
	);
}
