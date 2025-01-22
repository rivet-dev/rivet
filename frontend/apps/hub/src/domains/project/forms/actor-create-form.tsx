import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	createSchemaForm,
} from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { BuildSelect } from "../components/build-select";

export const formSchema = z.object({
	buildId: z.string(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Build = ({
	projectNameId,
	environmentNameId,
}: { projectNameId: string; environmentNameId: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="buildId"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Build</FormLabel>
					<FormControl>
						<BuildSelect
							projectNameId={projectNameId}
							environmentNameId={environmentNameId}
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
