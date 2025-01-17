import {
	Button,
	Flex,
	FormControl,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	JavaScriptCode,
	JsonCode,
	Label,
	createSchemaForm,
} from "@rivet-gg/components";
import { Icon, faTrash } from "@rivet-gg/icons";
import {
	type UseFormReturn,
	useFieldArray,
	useFormContext,
} from "react-hook-form";
import z from "zod";

const parsableJson = z.custom<string>((value) => {
	try {
		JSON.parse(value);
		return true;
	} catch {
		return false;
	}
}, "Invalid JSON");

export const formSchema = z.object({
	arguments: z.array(z.object({ code: parsableJson })),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, Reset } = createSchemaForm(formSchema);
export { Form, Submit };

export const Arguments = () => {
	const { control, register, setValue, formState } =
		useFormContext<FormValues>();
	const { fields, append, remove } = useFieldArray<FormValues>({
		name: "arguments",
		control,
	});

	return (
		<div className="border relative p-4 rounded-md mt-2">
			<Label className="inline-block absolute top-0 -translate-y-1/2 bg-card px-0.5 font-semibold">
				Arguments
			</Label>
			{fields.length === 0 ? <p className="mb-2">No arguments.</p> : null}
			{fields.map((field, index) => (
				<Flex
					key={field.id}
					gap="4"
					items="start"
					className="max-w-full w-full mb-4"
				>
					<FormFieldContext.Provider
						value={{ name: `arguments.${index}` }}
					>
						<FormItem
							flex="1"
							className="max-w-full w-full min-w-0 space-y-0 gap-2 flex flex-col"
						>
							<FormLabel>Argument #{index + 1}</FormLabel>
							<FormControl className="max-w-full w-full">
								<Flex
									gap="4"
									items="start"
									className="max-w-full w-full"
								>
									<JsonCode
										className="flex-1 min-w-0 max-w-full"
										{...register(`arguments.${index}`)}
										onChange={(value) => {
											setValue(
												`arguments.${index}.code`,
												value,
											);
										}}
									/>
									<Button
										type="button"
										variant="outline"
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
				variant="outline"
				type="button"
				onClick={() => append({ code: "" })}
			>
				Add argument
			</Button>
		</div>
	);
};

export const ExampleCall = ({ rpc }: { rpc: string }) => {
	const { watch } = useFormContext<FormValues>();

	const args = watch("arguments");
	return (
		<div className="border relative p-4 rounded-md">
			<Label className="inline-block absolute top-0 -translate-y-1/2 bg-card px-0.5 font-semibold">
				Constructed Call
			</Label>
			<JavaScriptCode
				value={`actor.${rpc}(\n${args.join(",\n")}\n)`}
				readOnly
				editable={false}
			/>
			<div className="mt-4 flex gap-2 justify-end">
				<Reset size="sm" variant="outline">
					Reset
				</Reset>
				<Submit size="sm">Call</Submit>
			</div>
		</div>
	);
};
