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
		projectId: z.string(),
	})
	.superRefine(async (arg, ctx) => {
		await safeAsyncValidation(ctx, async () => {
			const res =
				await rivetClient.cloud.games.namespaces.validateGameNamespace(
					arg.projectId,
					{
						displayName: arg.name,
						nameId: arg.slug || convertStringToId(arg.name),
					},
				);

			validateAgainstApi({
				group: "GAME_NAMESPACE",
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

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Name = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="name"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Name</FormLabel>
					<FormControl>
						<Input
							placeholder="Enter an environment name..."
							maxLength={25}
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Slug = () => {
	const { control, watch } = useFormContext<FormValues>();

	const name = watch("name");

	return (
		<FormField
			control={control}
			name="slug"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Slug</FormLabel>
					<FormControl>
						<Input
							placeholder={
								name
									? convertStringToId(name)
									: "Enter a slug..."
							}
							maxLength={25}
							{...field}
							onChange={(event) => {
								const value = event.target.value
									.replace(/[\s-]+/g, "-")
									.toLowerCase();
								field.onChange({ target: { value } });
							}}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
