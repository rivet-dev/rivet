import { useNavigate } from "@tanstack/react-router";
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

export const formSchema = z.object({
	org: z.string(),
	projectName: z.string().optional(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Organization = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	const navigate = useNavigate();
	return (
		<FormField
			control={control}
			name="org"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel>Organization</FormLabel>
					<FormControl>
						<CloudOrganizationSelect
							showCreateOrganization
							onCreateClick={() => {
								navigate({
									to: ".",
									search: {
										modal: "create-organization",
									},
								});
							}}
							onValueChange={field.onChange}
							value={field.value}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ProjectName = ({
	className,
	placeholder,
}: {
	className?: string;
	placeholder?: string;
}) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="projectName"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel>Project Name</FormLabel>
					<FormControl>
						<Input
							placeholder={placeholder || "My Rivet Project"}
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
