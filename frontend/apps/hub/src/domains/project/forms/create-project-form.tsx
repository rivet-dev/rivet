import { GroupSelect } from "@/domains/group/components/group-select";
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
import * as GroupCreateProjectForm from "./group-create-project-form";

export const formSchema = z.intersection(
	z.object({ developerGroupId: z.string().min(1, "Required") }),
	GroupCreateProjectForm.formSchema,
);

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Name = GroupCreateProjectForm.Name;
export const Slug = GroupCreateProjectForm.Slug;

export const Group = () => {
	const { setValue, control } = useFormContext<FormValues>();
	const { dialog, open, close } = useDialog.CreateGroup({
		onSuccess: (data) => {
			setValue("developerGroupId", data.groupId, {
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
				name="developerGroupId"
				render={({ field }) => (
					<FormItem>
						<FormLabel>Team</FormLabel>
						<GroupSelect
							showCreateGroup
							onValueChange={field.onChange}
							onCreateClick={open}
							value={field.value}
							defaultValue={`${field.value}`}
						/>
						<FormMessage />
					</FormItem>
				)}
			/>
		</>
	);
};
