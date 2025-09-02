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
import { useActorsView } from "../actors-view-context-provider";

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
	const { copy } = useActorsView();
	return (
		<FormField
			control={control}
			name="actorId"
			render={({ field }) => (
				<FormItem>
					<FormLabel>{copy.actorId}</FormLabel>
					<FormControl>
						<Input {...field} placeholder="Actor ID" />
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
