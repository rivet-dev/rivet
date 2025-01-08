import { ProjectSelect } from "@/domains/project/components/project-select";
import { useDialog } from "@/hooks/use-dialog";
import {
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	createSchemaForm,
} from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

export const formSchema = z.object({
	projectId: z.string().min(1, "Required"),
	token: z.string(),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, Reset } = createSchemaForm(formSchema);
export { Form, Submit, Reset };

export const Project = () => {
	const { control, setValue } = useFormContext<FormValues>();
	const { dialog, open, close } = useDialog.CreateProject({
		onSuccess: (data) => {
			setValue("projectId", data.gameId, {
				shouldDirty: true,
				shouldTouch: true,
			});
			close();
		},
	});

	return (
		<>
			{dialog}
			<FormField
				control={control}
				name="projectId"
				render={({ field }) => (
					<FormItem>
						<FormLabel>Project</FormLabel>
						<ProjectSelect
							showCreateProject
							onValueChange={field.onChange}
							value={field.value}
							defaultValue={`${field.value}`}
							onCreateClick={open}
						/>
						<FormMessage />
					</FormItem>
				)}
			/>
		</>
	);
};

export const Token = ({ value }: { value: string }) => {
	const { register } = useFormContext<FormValues>();
	return <input type="hidden" {...register("token", { value })} />;
};
