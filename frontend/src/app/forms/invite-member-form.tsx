import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import {
	createSchemaForm,
	FormControl,
	FormField,
	FormItem,
	FormMessage,
	Input,
} from "@/components";

export const formSchema = z.object({
	email: z.string().email("Invalid email address"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const EmailField = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="email"
			render={({ field }) => (
				// `space-y-0` so the (initially-empty) FormMessage slot
				// doesn't reserve / claim margin and shift the input downward
				// the moment the form transitions to a "dirty"/"touched"
				// state on focus.
				<FormItem className="space-y-0">
					<FormControl>
						<Input
							type="email"
							placeholder="colleague@company.com"
							autoComplete="off"
							{...field}
							style={{ scrollMarginTop: "1rem" }}
						/>
					</FormControl>
					<FormMessage className="mt-1.5" />
				</FormItem>
			)}
		/>
	);
};

export const RootError = () => {
	const { formState } = useFormContext<FormValues>();
	if (!formState.errors.root) return null;
	return (
		<p className="text-sm text-destructive mt-1">
			{formState.errors.root.message}
		</p>
	);
};
