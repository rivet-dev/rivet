import {
	Button,
	Flex,
	FormControl,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Text,
	createSchemaForm,
} from "@rivet-gg/components";
import { Icon, faTrash } from "@rivet-gg/icons";
import {
	type UseFormReturn,
	useFieldArray,
	useFormContext,
} from "react-hook-form";
import z from "zod";

export const formSchema = z.object({
	users: z
		.array(
			z.object({
				user: z.string().min(1),
				password: z.string(),
			}),
		)
		.superRefine((users, ctx) => {
			const userSet = new Set<string>();
			for (const [idx, user] of [...users].reverse().entries()) {
				if (userSet.has(user.user)) {
					ctx.addIssue({
						code: z.ZodIssueCode.custom,
						path: [idx, "user"],
						message: "Usernames must be unique",
					});
				}
				userSet.add(user.user);
			}
		}),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit } = createSchemaForm(formSchema);
export { Form, Submit };

export const Users = () => {
	const { control, register } = useFormContext<FormValues>();
	const { fields, append, remove } = useFieldArray({
		name: "users",
		control,
	});

	return (
		<>
			{fields.length === 0 ? <Text>There's no users added.</Text> : null}
			{fields.map((field, index) => (
				<Flex key={field.id} gap="4" items="start">
					<FormFieldContext.Provider
						value={{ name: `users.${index}.user` }}
					>
						<FormItem flex="1">
							<FormLabel>Username</FormLabel>
							<FormControl>
								<Input {...register(`users.${index}.user`)} />
							</FormControl>
							<FormMessage />
						</FormItem>
					</FormFieldContext.Provider>

					<FormFieldContext.Provider
						value={{ name: `users.${index}.password` }}
					>
						<FormItem flex="1">
							<FormLabel>Password</FormLabel>
							<FormControl>
								<Flex gap="4" items="start">
									<Input
										{...register(`users.${index}.password`)}
									/>
									<Button
										type="button"
										variant="secondary"
										size="icon"
										onClick={() => remove(index)}
									>
										<Icon icon={faTrash} />
									</Button>
								</Flex>
							</FormControl>
							<FormMessage />
						</FormItem>
					</FormFieldContext.Provider>
				</Flex>
			))}
			<Button
				variant="secondary"
				type="button"
				onClick={() => append({ user: "", password: "" })}
			>
				Add user
			</Button>
		</>
	);
};
