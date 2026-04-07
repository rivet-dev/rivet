import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "@/components/lib/create-schema-form";
import { FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Input } from "@/components/ui/input";

export const formSchema = z.object({
	name: z.string().min(1, "Name is required"),
	email: z.string().email("Invalid email address"),
	password: z.string().min(1, "Password is required"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const NameField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Name</FormLabel>
					<FormControl>
						<Input
							type="text"
							placeholder="Your name"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const EmailField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="email"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Email address</FormLabel>
					<FormControl>
						<Input
							type="email"
							placeholder="you@company.com"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const PasswordField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="password"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Password</FormLabel>
					<FormControl>
						<Input
							type="password"
							placeholder="Your password"
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
