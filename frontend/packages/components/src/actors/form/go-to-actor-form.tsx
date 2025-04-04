import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "../../lib/create-schema-form";
import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
} from "../../ui/form";
import { Input } from "../../ui/input";

export const formSchema = z.object({
	actorId: z.string().nonempty("Actor ID is required").uuid(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const ActorId = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="actorId"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Actor ID</FormLabel>
					<FormControl>
						<Input
							{...field}
							placeholder="00000000-0000-0000-0000-000000000000"
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
