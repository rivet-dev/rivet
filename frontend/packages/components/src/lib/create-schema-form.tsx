"use client";
import { zodResolver } from "@hookform/resolvers/zod";
import { Button, type ButtonProps, Form } from "@rivet-gg/components";
import { type ComponentProps, type ReactNode, useEffect } from "react";
import {
	type DefaultValues,
	type FieldPath,
	type FieldValues,
	type PathValue,
	type UseFormReturn,
	useForm,
	useFormContext,
	useFormState,
} from "react-hook-form";
import type z from "zod";

interface FormProps<FormValues extends FieldValues>
	extends Omit<ComponentProps<"form">, "onSubmit"> {
	onSubmit: SubmitHandler<FormValues>;
	defaultValues: DefaultValues<FormValues>;
	children: ReactNode;
}

type SubmitHandler<FormValues extends FieldValues> = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void> | void;

export const createSchemaForm = <Schema extends z.ZodSchema>(
	schema: Schema,
) => {
	return {
		Form: ({
			defaultValues,
			children,
			onSubmit,
			...props
		}: FormProps<z.TypeOf<Schema>>) => {
			const form = useForm<z.TypeOf<Schema>>({
				reValidateMode: "onSubmit",
				resolver: zodResolver(schema),
				defaultValues,
			});

			return (
				<Form {...form}>
					<form
						{...props}
						onSubmit={(event) => {
							event.stopPropagation();
							return form.handleSubmit(
								(values) => onSubmit(values, form),
								console.error,
							)(event);
						}}
						className="contents"
					>
						{children}
					</form>
				</Form>
			);
		},
		Submit: (props: ButtonProps & { disablePristine?: boolean }) => {
			const disablePristine = props.disablePristine ?? true;
			const { isSubmitting, isValidating, isDirty } =
				useFormState<z.TypeOf<Schema>>();
			return (
				<Button
					type="submit"
					disabled={disablePristine ? !isDirty : false}
					isLoading={isSubmitting || isValidating}
					{...props}
				/>
			);
		},
		Reset: (props: ButtonProps) => {
			const { defaultValues, isDirty } = useFormState<z.TypeOf<Schema>>();
			const { reset } = useFormContext<z.TypeOf<Schema>>();
			return (
				<Button
					type="button"
					disabled={!isDirty}
					onClick={() => reset(defaultValues)}
					{...props}
				/>
			);
		},
		SetValue: <Path extends FieldPath<z.TypeOf<Schema>>>(props: {
			name: Path;
			value: PathValue<z.TypeOf<Schema>, Path>;
		}) => {
			const { setValue, reset } = useFormContext<z.TypeOf<Schema>>();
			useEffect(() => {
				setValue(props.name, props.value, { shouldDirty: true });
				reset(
					{},
					{
						keepDirty: true,
						keepValues: true,
						keepDirtyValues: true,
					},
				);
			}, [props.name, setValue, reset, props.value]);
			return null;
		},
		useContext: () => useFormContext<z.TypeOf<Schema>>(),
	};
};
