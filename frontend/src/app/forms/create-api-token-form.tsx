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
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";

export const formSchema = z.object({
	name: z
		.string()
		.min(1, "Name is required")
		.max(255, "Name must be 255 characters or less"),
	expiresIn: z.string().optional(),
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

export const EXPIRATION_OPTIONS = [
	{ value: "15m", label: "15 minutes" },
	{ value: "1h", label: "1 hour" },
	{ value: "6h", label: "6 hours" },
	{ value: "12h", label: "12 hours" },
	{ value: "1d", label: "1 day" },
	{ value: "7d", label: "7 days" },
	{ value: "30d", label: "30 days" },
	{ value: "90d", label: "90 days" },
	{ value: "1y", label: "1 year" },
	{ value: "never", label: "Never" },
] as const;

export const ExpiresIn = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="expiresIn"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel>Expiration</FormLabel>
					<Select onValueChange={field.onChange} value={field.value}>
						<FormControl>
							<SelectTrigger>
								<SelectValue placeholder="Select expiration duration" />
							</SelectTrigger>
						</FormControl>
						<SelectContent>
							{EXPIRATION_OPTIONS.map((option) => (
								<SelectItem key={option.value} value={option.value}>
									{option.label}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
