import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	createSchemaForm,
} from "@rivet-gg/components";
import { Switch } from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";
import { ActorTagsSelect } from "../actor-tags-select";

export const formSchema = z.object({
	tags: z.record(z.string()),
	showDestroyed: z.boolean().default(true),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, Reset } = createSchemaForm(formSchema);
export { Form, Submit, Reset };

export const Tags = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="tags"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Tags</FormLabel>
					<FormControl>
						<ActorTagsSelect
							value={field.value}
							onValueChange={field.onChange}
							showSelectedOptions={1}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export function ShowDestroyed() {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="showDestroyed"
			render={({ field }) => (
				<FormItem className="space-y-0">
					<div className="flex justify-between items-center">
						<FormLabel>Show destroyed?</FormLabel>
						<FormControl>
							<Switch
								className="mt-0"
								{...field}
								checked={field.value}
								onCheckedChange={field.onChange}
								value="show-destroyed"
							/>
						</FormControl>
					</div>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
}
