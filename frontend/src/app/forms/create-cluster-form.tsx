import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	CloudOrganizationSelect,
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";

// Cluster names are DNS labels: they appear verbatim in BYOC API paths.
export const formSchema = z.object({
	name: z
		.string()
		.regex(/^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$/, {
			message:
				"Name must be lowercase letters, numbers, and dashes, and start and end with a letter or number",
		}),
	organization: z.string().nonempty("Organization is required"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Name = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Name</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="Enter a cluster name..."
							autoFocus
							autoComplete="off"
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Organization = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="organization"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Organization</FormLabel>
					<FormControl className="row-start-2">
						<CloudOrganizationSelect
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const DefaultSubmit = () => {
	return <Submit type="submit">Create Cluster</Submit>;
};
