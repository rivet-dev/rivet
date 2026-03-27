import { faTrash, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	type FieldArrayPath,
	type Path,
	type PathValue,
	useFieldArray,
	useFormContext,
} from "react-hook-form";
import z from "zod";
import {
	Button,
	Combobox,
	createSchemaForm,
	FormControl,
	FormDescription,
	FormField,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Label,
} from "@/components";
import { useCloudNamespaceDataProvider } from "@/components/actors";
import { Slider } from "@/components/ui/slider";

export const formSchema = z
	.object({
		image: z.object({
			repository: z.string().min(1, "Repository is required"),
			tag: z
				.string()
				.nonempty("Tag is required when a repository is selected")
				.optional(),
		}),
		minCount: z.coerce.number().min(0),
		maxCount: z.coerce.number().min(1),
		environment: z.array(z.tuple([z.string(), z.string()])).default([]),
		command: z.string().optional(),
		args: z.string().optional(),
	})
	.superRefine((data, ctx) => {
		console.log(data);
		if (data.image.repository && !!data.image.tag) {
			ctx.addIssue({
				code: z.ZodIssueCode.custom,
				message: "Tag is required when a repository is selected",
				path: ["image", "tag"],
			});
		}
	});

export type FormValues = z.infer<typeof formSchema>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);

export { Form, SetValue, Submit };

export const Image = () => {
	const dataProvider = useCloudNamespaceDataProvider();
	const { control, setValue, watch } = useFormContext<FormValues>();

	const selectedRepository = watch("image.repository");

	const {
		data: repositories,
		hasNextPage: hasMoreRepositories,
		fetchNextPage: fetchNextRepositoryPage,
		isLoading: isLoadingRepositories,
	} = useInfiniteQuery({
		...dataProvider.currentProjectImageRepositoriesQueryOptions(),
	});

	const {
		data: tags,
		hasNextPage: hasMoreTags,
		fetchNextPage: fetchNextTagPage,
		isLoading: isLoadingTags,
	} = useInfiniteQuery({
		...dataProvider.currentProjectTagsQueryOptions({
			repository: selectedRepository,
		}),
		enabled: !!selectedRepository,
	});

	return (
		<div className="space-y-2">
			<FormLabel asChild>
				<p>Image</p>
			</FormLabel>
			<div className="grid grid-cols-2 gap-2">
				<FormField
					control={control}
					name="image.repository"
					render={({ field }) => (
						<FormItem className="flex items-center gap-2 space-y-0">
							<FormLabel>Repository</FormLabel>
							<FormControl>
								<Combobox
									className="mt-0"
									value={field.value}
									onValueChange={(value) => {
										field.onChange(value);
										setValue("image.tag", "", {
											shouldDirty: true,
											shouldTouch: true,
										});
									}}
									placeholder="Select repository"
									isLoading={isLoadingRepositories}
									options={
										repositories?.map((repo) => ({
											value: repo.repository,
											label: repo.repository,
										})) ?? []
									}
									onLoadMore={
										hasMoreRepositories
											? fetchNextRepositoryPage
											: undefined
									}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
				<FormField
					control={control}
					name="image.tag"
					render={({ field }) => (
						<FormItem className="flex items-center gap-2 space-y-0">
							<FormLabel>Tag</FormLabel>
							<FormControl>
								<Combobox
									value={field.value}
									onValueChange={field.onChange}
									placeholder={
										selectedRepository
											? "Select tag"
											: "Select a repository first"
									}
									isLoading={
										!!selectedRepository && isLoadingTags
									}
									options={
										selectedRepository && tags
											? tags.map((tag) => ({
													value: tag.tag,
													label: tag.tag,
												}))
											: []
									}
									onLoadMore={
										hasMoreTags
											? fetchNextTagPage
											: undefined
									}
								/>
							</FormControl>
							<FormMessage />
						</FormItem>
					)}
				/>
			</div>
		</div>
	);
};

export const MinMaxCount = () => {
	const { setValue, watch } = useFormContext<FormValues>();
	const minCount = watch("minCount") ?? 0;
	const maxCount = watch("maxCount") ?? 1;

	return (
		<div className="space-y-4">
			<div className="flex justify-between items-center">
				<FormLabel asChild>
					<p>Instance Count</p>
				</FormLabel>
				<span className="text-sm text-muted-foreground">
					{minCount} – {maxCount}
				</span>
			</div>
			<FormDescription>
				Minimum and maximum number of instances to run.
			</FormDescription>
			<Slider
				min={0}
				max={100_000}
				step={1}
				value={[minCount, maxCount]}
				onValueChange={([min, max]) => {
					setValue("minCount", min, {
						shouldDirty: true,
						shouldTouch: true,
						shouldValidate: true,
					});
					setValue("maxCount", max ?? min + 1, {
						shouldDirty: true,
						shouldTouch: true,
						shouldValidate: true,
					});
				}}
			/>
			<div className="flex justify-between text-xs text-muted-foreground">
				<span>0</span>
				<span>100,000</span>
			</div>
		</div>
	);
};

export const Environment = <TValues extends Record<string, any> = FormValues>({
	name = "environment" as FieldArrayPath<TValues>,
}: {
	name?: FieldArrayPath<TValues>;
}) => {
	const { control, setValue, watch } = useFormContext<TValues>();
	const { fields, append, remove } = useFieldArray<TValues>({
		name,
		control,
	});

	return (
		<div className="space-y-2">
			<FormLabel asChild>
				<p>Environment Variables</p>
			</FormLabel>
			<FormDescription>
				Environment variables passed to the running instances.
			</FormDescription>
			<div className="grid grid-cols-[1fr,1fr,auto] items-start gap-2 empty:hidden">
				{fields.length > 0 ? (
					<>
						<Label asChild>
							<p>Key</p>
						</Label>
						<Label asChild>
							<p>Value</p>
						</Label>
						<p />
					</>
				) : null}
				{fields.map((field, index) => (
					<div
						key={field.id}
						className="grid grid-cols-subgrid col-span-full"
					>
						<FormFieldContext.Provider
							value={{ name: `${name}.${index}.0` }}
						>
							<FormItem className="grid grid-cols-subgrid grid-rows-subgrid row-span-full">
								<FormLabel aria-hidden hidden>
									Key
								</FormLabel>
								<FormControl>
									<Input
										placeholder="KEY"
										className="w-full font-mono"
										value={watch(
											`${name}.${index}.0` as Path<TValues>,
										)}
										onChange={(e) => {
											setValue(
												`${name}.${index}.0` as Path<TValues>,
												e.target.value as PathValue<
													TValues,
													Path<TValues>
												>,
												{
													shouldDirty: true,
													shouldTouch: true,
													shouldValidate: true,
												},
											);
										}}
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						</FormFieldContext.Provider>

						<FormFieldContext.Provider
							value={{ name: `${name}.${index}.1` }}
						>
							<FormItem className="grid grid-cols-subgrid grid-rows-subgrid row-span-full">
								<FormLabel aria-hidden hidden>
									Value
								</FormLabel>
								<FormControl>
									<Input
										placeholder="value"
										className="w-full"
										value={watch(
											`${name}.${index}.1` as Path<TValues>,
										)}
										onChange={(e) => {
											setValue(
												`${name}.${index}.1` as Path<TValues>,
												e.target.value as PathValue<
													TValues,
													Path<TValues>
												>,
												{
													shouldDirty: true,
													shouldTouch: true,
													shouldValidate: true,
												},
											);
										}}
									/>
								</FormControl>
								<FormMessage />
							</FormItem>
						</FormFieldContext.Provider>

						<Button
							size="icon"
							className="self-end row-start-1"
							variant="secondary"
							type="button"
							onClick={() => remove(index)}
						>
							<Icon icon={faTrash} />
						</Button>
					</div>
				))}
			</div>
			<Button
				className="justify-self-start"
				variant="secondary"
				size="sm"
				type="button"
				onClick={() =>
					append([["", ""]] as PathValue<TValues, Path<TValues>>)
				}
			>
				Add variable
			</Button>
		</div>
	);
};

export const Command = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="command"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Command</FormLabel>
					<FormControl>
						<Input
							placeholder="/app/server"
							className="font-mono"
							{...field}
						/>
					</FormControl>
					<FormDescription>
						Override the container entrypoint command.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};

export const Args = () => {
	const { control } = useFormContext<FormValues>();
	return (
		<FormField
			control={control}
			name="args"
			render={({ field }) => (
				<FormItem>
					<FormLabel>Arguments</FormLabel>
					<FormControl>
						<Input
							placeholder="--port 8080 --workers 4"
							className="font-mono"
							{...field}
						/>
					</FormControl>
					<FormDescription>
						Space-separated arguments passed to the command.
					</FormDescription>
					<FormMessage />
				</FormItem>
			)}
		/>
	);
};
