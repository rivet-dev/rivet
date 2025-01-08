import {
	Button,
	Flex,
	FormControl,
	FormDescription,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Strong,
	Switch,
	Text,
	createSchemaForm,
} from "@rivet-gg/components";
import { Icon, faTrash } from "@rivet-gg/icons";
import {
	type Control,
	type UseFormReturn,
	useController,
	useFieldArray,
	useFormContext,
	useWatch,
} from "react-hook-form";
import z from "zod";

export const formSchema = z.object({
	variables: z
		.array(
			z.object({
				key: z.string().min(1),
				value: z.string(),
				isSecret: z.boolean().default(false),
			}),
		)
		.superRefine((variables, ctx) => {
			const variablesSet = new Set<string>();
			for (const [idx, variable] of [...variables].reverse().entries()) {
				if (variablesSet.has(variable.key)) {
					ctx.addIssue({
						code: z.ZodIssueCode.custom,
						path: [idx, "key"],
						message: "Variable keys must be unique",
					});
				}
				variablesSet.add(variable.key);
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

export const Variables = () => {
	const { control, register } = useFormContext<FormValues>();
	const { fields, append, remove } = useFieldArray({
		name: "variables",
		control,
	});

	return (
		<>
			{fields.length === 0 ? (
				<Text mb="2">There's no variables added.</Text>
			) : null}
			<Button
				variant="secondary"
				type="button"
				onClick={() => append({ value: "", key: "", isSecret: false })}
			>
				Add variable
			</Button>
			{fields.map((field, index) => (
				<Flex key={field.id} gap="4" items="start" mt="2">
					<FormFieldContext.Provider
						value={{ name: `variables.${index}.key` }}
					>
						<FormItem flex="1">
							<FormLabel>Key</FormLabel>
							<FormControl>
								<Input
									{...register(`variables.${index}.key`)}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					</FormFieldContext.Provider>

					<FormFieldContext.Provider
						value={{ name: `variables.${index}.value` }}
					>
						<FormItem flex="1">
							<FormLabel>Value</FormLabel>
							<FormControl>
								<Input
									{...register(`variables.${index}.value`)}
								/>
							</FormControl>
							<EncryptInformation
								index={index}
								control={control}
							/>
							<FormMessage />
						</FormItem>
					</FormFieldContext.Provider>

					<FormFieldContext.Provider
						value={{ name: `variables.${index}.isSecret` }}
					>
						<Encrypt index={index} onRemove={() => remove(index)} />
					</FormFieldContext.Provider>
				</Flex>
			))}
		</>
	);
};

const EncryptInformation = ({
	control,
	index,
}: { control: Control<FormValues>; index: number }) => {
	const isSecret = useWatch<FormValues>({
		control,
		name: `variables.${index}.isSecret`,
	});
	return (
		<FormDescription>
			{isSecret ? (
				<>
					This value <Strong>will no longer be viewable</Strong> once
					saved.
				</>
			) : null}
		</FormDescription>
	);
};

function Encrypt({ index, onRemove }: { index: number; onRemove: () => void }) {
	const {
		field: { onChange, value },
	} = useController<FormValues>({
		name: `variables.${index}.isSecret` as const,
	});

	return (
		<FormItem>
			<FormLabel>Encrypt</FormLabel>
			<FormControl>
				<Flex gap="4">
					<Switch
						checked={value as boolean}
						onCheckedChange={onChange}
					/>
					<Button
						type="button"
						variant="secondary"
						size="icon"
						onClick={onRemove}
					>
						<Icon icon={faTrash} />
					</Button>
				</Flex>
			</FormControl>
			<FormMessage />
		</FormItem>
	);
}
