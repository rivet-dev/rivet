import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
} from "@/components";

export const formSchema = z.object({
	name: z
		.string()
		.min(1, "Name is required")
		.max(255, "Name must be 255 characters or less"),
	expiresAt: z.string().optional(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Name = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel>Name</FormLabel>
					<FormControl>
						<Input
							placeholder="e.g., CI/CD Pipeline, Production Server"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ExpiresAt = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="expiresAt"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel>Expiration (Optional)</FormLabel>
					<FormControl>
						<Input type="datetime-local" {...field} />
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
