import { bootstrapQueryOptions } from "@/domains/auth/queries/bootstrap";
import { queryClient } from "@/queries/global";
import {
	Button,
	Combobox,
	createSchemaForm,
	FormControl,
	FormField,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	type ComboboxOption,
	FormDescription,
	Checkbox,
	Code,
} from "@rivet-gg/components";
import { Icon, faTrash } from "@rivet-gg/icons";
import {
	type UseFormReturn,
	useFieldArray,
	useFormContext,
} from "react-hook-form";
import z from "zod";

export const formSchema = z.object({
	stripPrefix: z.boolean().optional(),
	routeSubpaths: z.boolean().optional(),
	hostname: z
		.string()
		.min(1)
		.refine((value) => {
			const regex = /^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/;
			return regex.test(value);
		}, "Hostname must be a valid domain name")
		.refine(
			async (value) => {
				const bootstrap = queryClient.getQueryData(
					bootstrapQueryOptions().queryKey,
				);

				const domain = bootstrap?.domains.job || "rivet-job.local";

				return value.endsWith(`.${domain}`);
			},
			() => {
				const bootstrap = queryClient.getQueryData(
					bootstrapQueryOptions().queryKey,
				);

				const domain = bootstrap?.domains.job || "rivet-job.local";
				return {
					message: `Hostname must end with .${domain}`,
				};
			},
		),
	path: z
		.string()
		.min(1)
		.refine((value) => {
			const regex = /^(\/[a-zA-Z0-9-_]+)+$/;
			return regex.test(value);
		}, "Path must start with a / and contain only alphanumeric characters, dashes, and underscores")
		.refine((value) => {
			const endsWithSlash = value.endsWith("/");
			return !endsWithSlash;
		}, "Path must not end with a /"),
	tags: z
		.array(
			z.object({
				key: z.string().min(1),
				value: z.string(),
			}),
		)
		.min(1, "At least one selector is required")
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

export const Hostname = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="hostname"
			render={({ field }) => (
				<FormItem className="flex-1">
					<FormLabel>Hostname</FormLabel>
					<FormControl>
						<Input
							type="text"
							className="input"
							placeholder="example.com"
							{...field}
						/>
					</FormControl>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Path = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="path"
			render={({ field, fieldState }) => (
				<FormItem className="flex-1">
					<FormLabel>Path</FormLabel>
					<FormControl>
						<Input
							type="text"
							className="input"
							placeholder="/api/v1"
							{...field}
						/>
					</FormControl>
					<FormMessage />
					<FormDescription>
						<PathMessage />
					</FormDescription>
				</FormItem>
			)}
		/>
	);
};

const PathMessage = () => {
	const { watch } = useFormContext<FormValues>();

	const path = watch("path");

	return path.endsWith("/*") ? (
		<span>Maximum of 8 path components when routing path prefixes.</span>
	) : null;
};

export const Tags = ({
	onCreateKeyOption,
	onCreateValueOption,
	keys,
	values,
}: {
	onCreateKeyOption: (option: string) => void;
	onCreateValueOption: (option: string) => void;
	keys: ComboboxOption[];
	values: ComboboxOption[];
}) => {
	const { control, setValue, watch } = useFormContext<FormValues>();
	const { fields, append, remove } = useFieldArray({
		name: "tags",
		control,
	});

	return (
		<>
			{fields.length === 0 ? (
				<p className="text-xs mb-2">There's no selectors added.</p>
			) : null}
			{fields.map((field, index) => (
				<div
					key={field.id}
					className="grid grid-cols-[1fr,1fr,auto] grid-rows-[repeat(3,auto)] items-start mt-2 gap-2"
				>
					<FormFieldContext.Provider
						value={{ name: `tags.${index}.key` }}
					>
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
									value={watch(`tags.${index}.key`)}
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

					<FormFieldContext.Provider
						value={{ name: `tags.${index}.value` }}
					>
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
									value={watch(`tags.${index}.value`)}
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
						onClick={() => remove(index)}
					>
						<Icon icon={faTrash} />
					</Button>
				</div>
			))}
			<Button
				className="justify-self-start"
				variant="secondary"
				size="sm"
				type="button"
				onClick={() => append({ value: "", key: "" })}
			>
				Add a selector
			</Button>
		</>
	);
};

export const StripPrefix = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="stripPrefix"
			render={({ field }) => (
				<FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4 shadow">
					<FormControl>
						<Checkbox
							checked={field.value}
							onCheckedChange={field.onChange}
						/>
					</FormControl>
					<FormLabel asChild>
						{/* biome-ignore lint/a11y/noLabelWithoutControl: injected by FromLabel */}
						<label className="space-y-1 leading-none cursor-pointer">
							<p>Strip Prefix</p>
							<FormDescription>
								If enabled, the matching route prefix will be
								removed from request paths before forwarding.
								For example, with route <Code>/a/b</Code>, a
								request to
								<Code>/a/b/c</Code> becomes <Code>/c</Code>. If
								disabled, the full original path remains intact.
							</FormDescription>
						</label>
					</FormLabel>
				</FormItem>
			)}
		/>
	);
};

export const RouteSubpaths = () => {
	const { control } = useFormContext<FormValues>();

	return (
		<FormField
			control={control}
			name="routeSubpaths"
			render={({ field }) => (
				<FormItem className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4 shadow">
					<FormControl>
						<Checkbox
							checked={field.value}
							onCheckedChange={field.onChange}
						/>
					</FormControl>
					<FormLabel asChild>
						{/* biome-ignore lint/a11y/noLabelWithoutControl: injected by FromLabel */}
						<label className="space-y-1 leading-none cursor-pointer">
							<p>Route Subpaths</p>
							<FormDescription>
								If enabled, a route pattern <Code>/a/b</Code>{" "}
								will match both the exact path <Code>/a/b</Code>{" "}
								and any nested paths like <Code>/a/b/c</Code>.
								If disabled, only exact matches to{" "}
								<Code>/a/b</Code> will be routed.
							</FormDescription>
						</label>
					</FormLabel>
				</FormItem>
			)}
		/>
	);
};
