import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "@/components/lib/create-schema-form";
import { FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

export const formSchema = z
	.object({
		newPassword: z.string().min(8, "Password must be at least 8 characters"),
		confirmPassword: z.string().min(1, "Please confirm your password"),
	})
	.refine((data) => data.newPassword === data.confirmPassword, {
		message: "Passwords do not match",
		path: ["confirmPassword"],
	});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const NewPasswordField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="newPassword"
			render={({ field }) => (
				<FormItem>
					<FormLabel>New password</FormLabel>
					<FormControl>
						<Input
							type="password"
							placeholder="At least 8 characters"
							autoComplete="new-password"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const ConfirmPasswordField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="confirmPassword"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Confirm password</FormLabel>
					<FormControl>
						<Input
							type="password"
							placeholder="Repeat your new password"
							autoComplete="new-password"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const RootError = () => {
	const { formState } = useFormContext<FormValues>();
	if (!formState.errors.root) return null;
	return (
		<p className="text-sm text-destructive">
			{formState.errors.root.message}
		</p>
	);
};
