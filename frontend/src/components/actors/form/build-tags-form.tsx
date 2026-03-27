import { faTrash, Icon } from "@rivet-gg/icons";
import {
	type UseFormReturn,
	useFieldArray,
	useFormContext,
	useWatch,
} from "react-hook-form";
import z from "zod";
import { createSchemaForm } from "../../lib/create-schema-form";
import { Button } from "../../ui/button";
import { Combobox, type ComboboxOption as Option } from "../../ui/combobox";
import {
	FormControl,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
} from "../../ui/form";
import { Text } from "../../ui/typography";

export const formSchema = z.object({
	tags: z
		.array(
			z.object({
				key: z.string().min(1),
				value: z.string(),
			}),
		)
		.superRefine((tags, ctx) => {
			const tagsSet = new Set<string>();
			for (const [idx, tag] of [...tags].reverse().entries()) {
				if (tagsSet.has(tag.key)) {
					ctx.addIssue({
						code: z.ZodIssueCode.custom,
						path: [idx, "key"],
						message: "Tag keys must be unique",
					});
				}
				tagsSet.add(tag.key);
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

interface TagFieldProps {
	index: number;
	onCreateKeyOption: (option: string) => void;
	onCreateValueOption: (option: string) => void;
	keys: Option[];
	values: Option[];
	onRemove: () => void;
}

const TagField = ({
	index,
	onCreateKeyOption,
	onCreateValueOption,
	keys,
	values,
	onRemove,
}: TagFieldProps) => {
	const { setValue } = useFormContext<FormValues>();
	const keyValue = useWatch<FormValues>({ name: `tags.${index}.key` as "tags.0.key" });
	const tagValue = useWatch<FormValues>({ name: `tags.${index}.value` as "tags.0.value" });

	return (
		<div className="grid grid-cols-[1fr,1fr,auto] grid-rows-[repeat(3,auto)] items-start mt-2 gap-2">
			<FormFieldContext.Provider value={{ name: `tags.${index}.key` }}>
				<FormItem
					flex="1"
					className="grid grid-cols-subgrid grid-rows-subgrid row-span-full"
				>
					<FormLabel>Key</FormLabel>
					<FormControl>
						<Combobox
							placeholder="Choose a key"
							options={keys}
							className="w-full"
							value={keyValue as string}
							onValueChange={(value) => {
								setValue(`tags.${index}.key`, value, {
									shouldDirty: true,
									shouldTouch: true,
									shouldValidate: true,
								});
							}}
							allowCreate
							onCreateOption={onCreateKeyOption}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			</FormFieldContext.Provider>

			<FormFieldContext.Provider value={{ name: `tags.${index}.value` }}>
				<FormItem
					flex="1"
					className="grid grid-cols-subgrid grid-rows-subgrid row-span-full"
				>
					<FormLabel>Value</FormLabel>
					<FormControl>
						<Combobox
							placeholder="Choose a value"
							options={values}
							className="w-full"
							value={tagValue as string}
							onValueChange={(value) => {
								setValue(`tags.${index}.value`, value, {
									shouldDirty: true,
									shouldTouch: true,
									shouldValidate: true,
								});
							}}
							allowCreate
							onCreateOption={onCreateValueOption}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			</FormFieldContext.Provider>
			<Button
				size="icon"
				className="self-end row-start-2"
				variant="secondary"
				type="button"
				onClick={onRemove}
			>
				<Icon icon={faTrash} />
			</Button>
		</div>
	);
};

export const Tags = ({
	onCreateKeyOption,
	onCreateValueOption,
	keys,
	values,
}: {
	onCreateKeyOption: (option: string) => void;
	onCreateValueOption: (option: string) => void;
	keys: Option[];
	values: Option[];
}) => {
	const { control } = useFormContext<FormValues>();
	const { fields, append, remove } = useFieldArray({
		name: "tags",
		control,
	});

	return (
		<>
			{fields.length === 0 ? (
				<Text mb="2" className="text-xs">
					There's no tags added.
				</Text>
			) : null}
			{fields.map((field, index) => (
				<TagField
					key={field.id}
					index={index}
					onCreateKeyOption={onCreateKeyOption}
					onCreateValueOption={onCreateValueOption}
					keys={keys}
					values={values}
					onRemove={() => remove(index)}
				/>
			))}
			<Button
				className="justify-self-start"
				variant="secondary"
				size="sm"
				type="button"
				onClick={() => append({ value: "", key: "" })}
			>
				Add a tag
			</Button>
		</>
	);
};
