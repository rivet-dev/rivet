import {
	type FieldPath,
	type UseFormReturn,
	useFormContext,
} from "react-hook-form";
import z from "zod";
import {
	Checkbox,
	createSchemaForm,
	FormField,
	FormMessage,
} from "@/components";
import * as SingleRunnerConfigForm from "./edit-shared-runner-config-form";

export const formSchema = z.object({
	datacenters: z
		.record(
			z.string(),
			SingleRunnerConfigForm.formSchema
				.omit({ regions: true })
				.and(z.object({ enable: z.boolean().optional() }))
				.optional(),
		)
		.optional()
		.refine((obj) => {
			return Object.values(obj || {}).some(
				(dcConfig) => dcConfig?.enable,
			);
		}, "At least one datacenter must be enabled."),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Enable = <TValues extends Record<string, any> = FormValues>({
	name,
}: {
	name: FieldPath<TValues>;
}) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name={name as FieldPath<FormValues>}
			render={({ field }) => (
				<Checkbox
					onClick={(e) => {
						e.stopPropagation();
					}}
					checked={(field.value as unknown as boolean) ?? false}
					name={field.name}
					onCheckedChange={field.onChange}
				/>
			)}
		/>
	);
};

export const Datacenters = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="datacenters"
			render={() => <FormMessage />}
		/>
	);
};

export const Url = SingleRunnerConfigForm.Url<FormValues>;
export const MaxRunners = SingleRunnerConfigForm.MaxRunners<FormValues>;
export const MinRunners = SingleRunnerConfigForm.MinRunners<FormValues>;
export const RequestLifespan =
	SingleRunnerConfigForm.RequestLifespan<FormValues>;
export const RunnersMargin = SingleRunnerConfigForm.RunnersMargin<FormValues>;
export const SlotsPerRunner = SingleRunnerConfigForm.SlotsPerRunner<FormValues>;
export const Headers = SingleRunnerConfigForm.Headers<FormValues>;
