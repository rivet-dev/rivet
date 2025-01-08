import {
	safeAsyncValidation,
	validateAgainstApi,
} from "@/lib/async-validation";
import { convertStringToId } from "@/lib/utils";
import { rivetClient } from "@/queries/global";
import {
	FormControl,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	createSchemaForm,
} from "@rivet-gg/components";
import { type UseFormReturn, useFormContext } from "react-hook-form";
import z from "zod";

export const formSchema = z
	.object({
		name: z.string().max(25),
		slug: z.string().max(25).optional(),
	})
	.superRefine(async (arg, ctx) => {
		await safeAsyncValidation(ctx, async () => {
			const res = await rivetClient.cloud.games.validateGame({
				displayName: arg.name,
				nameId: arg.slug || convertStringToId(arg.name),
			});

			validateAgainstApi({
				group: "GAME",
				errors: res.errors,
			}).setSchemaIssues(ctx, {
				name: "display-name",
				slug: "name-id",
			});
		});

		return z.NEVER;
	});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Name = ({ className }: { className?: string }) => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Project Name</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="Enter a project name..."
							maxLength={25}
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Slug = ({ className }: { className?: string }) => {
	const { control, watch } = useFormContext<FormValues>();

	const name = watch("name");

	return (
		<FormField
			control={control}
			name="slug"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-2">Slug</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder={
								name
									? convertStringToId(name)
									: "Enter a slug..."
							}
							maxLength={25}
							{...field}
							onChange={(event) => {
								const value = convertStringToId(
									event.target.value,
								);
								field.onChange({ target: { value } });
							}}
						/>
					</FormControl>
					<FormMessage className="col-span-2" />
				</FormItem>
			)}
		/>
	);
};
