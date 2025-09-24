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
	token: z.string().nonempty("Token is required"),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Token = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="token"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Token</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="Enter a token..."
							type="password"
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};
