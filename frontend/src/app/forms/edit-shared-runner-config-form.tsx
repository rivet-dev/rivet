import { faTrash, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	type FieldArrayPath,
	type FieldPath,
	type Path,
	type PathValue,
	type UseFormReturn,
	useFieldArray,
	useFormContext,
} from "react-hook-form";
import z from "zod";
import {
	Button,
	Checkbox,
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
import { ActorRegion, useEngineCompatDataProvider } from "@/components/actors";
import { VisibilitySensor } from "@/components/visibility-sensor";

export const formSchema = z.object({
	url: z.string().url(),
	maxRunners: z.coerce.number().positive(),
	minRunners: z.coerce.number().min(0),
	requestLifespan: z.coerce.number().positive(),
	runnersMargin: z.coerce.number().min(0),
	slotsPerRunner: z.coerce.number().positive(),
	headers: z.array(z.array(z.string())).default([]),
	regions: z
		.record(z.string(), z.boolean().optional())
		.optional()
		.refine((obj) => {
			return Object.values(obj || {}).some((v) => v);
		}, "At least one region must be selected."),
});

export type FormValues = z.infer<typeof formSchema>;
export type SubmitHandler = (
	values: FormValues,
	form: UseFormReturn<FormValues>,
) => Promise<void>;

const { Form, Submit, SetValue } = createSchemaForm(formSchema);
export { Form, Submit, SetValue };

export const Url = <TValues extends Record<string, any> = FormValues>({
	name = "url" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Endpoint</FormLabel>
					<FormControl className="row-start-2">
						<Input
							placeholder="https://your-rivet-runner"
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const MinRunners = <TValues extends Record<string, any> = FormValues>({
	name = "minRunners" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Min Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormDescription className="col-span-1">
						The minimum number of runners to keep running.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const MaxRunners = <TValues extends Record<string, any> = FormValues>({
	name = "maxRunners" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Max Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>

					<FormDescription className="col-span-1">
						The maximum number of runners that can be created to
						handle load.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const RequestLifespan = <
	TValues extends Record<string, any> = FormValues,
>({
	name = "requestLifespan" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Request Lifespan
					</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormDescription className="col-span-1">
						The maximum duration (in seconds) a request can take
						before being terminated.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const RunnersMargin = <
	TValues extends Record<string, any> = FormValues,
>({
	name = "runnersMargin" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Runners Margin</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormDescription className="col-span-1">
						The number of extra runners to keep running to handle
						sudden spikes in load.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const SlotsPerRunner = <
	TValues extends Record<string, any> = FormValues,
>({
	name = "slotsPerRunner" as FieldPath<TValues>,
	className,
}: {
	name?: FieldPath<TValues>;
	className?: string;
}) => {
	const { control } = useFormContext<TValues>();
	return (
		<FormField
			control={control}
			name={name}
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Slots Per Runner
					</FormLabel>
					<FormControl className="row-start-2">
						<Input type="number" {...field} />
					</FormControl>
					<FormDescription className="col-span-1">
						The number of concurrent slots each runner can handle.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Headers = <TValues extends Record<string, any> = FormValues>({
	name = "headers" as FieldArrayPath<TValues>,
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
				<p>Custom Headers</p>
			</FormLabel>
			<FormDescription>
				Custom headers to add to each request to the runner. Useful for
				providing authentication or other information.
			</FormDescription>
			<div className="grid grid-cols-[1fr,1fr,auto] grid-rows-[repeat(3,auto)] items-start gap-2 empty:hidden">
				{fields.length > 0 ? (
					<>
						<Label asChild>
							<p>Name</p>
						</Label>
						<Label asChild>
							<p>Value</p>
						</Label>
						<p></p>
					</>
				) : null}
				{fields.map((field, index) => (
					<div
						key={field.id}
						className="grid grid-cols-subgrid grid-rows-
col-span-full flex-1"
					>
						<FormFieldContext.Provider
							value={{ name: `${name}.${index}.0` }}
						>
							<FormItem
								flex="1"
								className="grid grid-cols-subgrid grid-rows-subgrid row-span-full"
							>
								<FormLabel aria-hidden hidden>
									Key
								</FormLabel>
								<FormControl>
									<Input
										placeholder="Enter a value"
										className="w-full"
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
							<FormItem
								flex="1"
								className="grid grid-cols-subgrid grid-rows-subgrid row-span-full"
							>
								<FormLabel aria-hidden hidden>
									Value
								</FormLabel>
								<FormControl>
									<Input
										placeholder="Enter a value"
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
				Add a header
			</Button>
		</div>
	);
};

export const Regions = () => {
	const { control } = useFormContext<FormValues>();
	const { data, hasNextPage, fetchNextPage } = useInfiniteQuery({
		...useEngineCompatDataProvider().regionsQueryOptions(),
		maxPages: Infinity,
	});

	return (
		<div className="space-y-2">
			<FormLabel asChild>
				<p>Datacenters</p>
			</FormLabel>
			<FormDescription>
				Datacenters where this provider can deploy actors.
			</FormDescription>
			<div className="space-y-4">
				{data?.map((region) => (
					<FormField
						key={region.id}
						control={control}
						name={`regions.${region.id}`}
						render={({ field }) => (
							<>
								<div className="flex items-start gap-3">
									<Checkbox
										id={`region-${region.id}`}
										checked={field.value ?? false}
										name={field.name}
										onCheckedChange={field.onChange}
									/>
									<div className="grid gap-2">
										<Label htmlFor={`region-${region.id}`}>
											<ActorRegion
												regionId={region.id}
												showLabel
											/>
										</Label>
									</div>
								</div>
								<FormMessage />
							</>
						)}
					/>
				))}
				{hasNextPage ? (
					<VisibilitySensor onChange={fetchNextPage} />
				) : null}
			</div>{" "}
			<FormField
				control={control}
				name="regions"
				render={() => <FormMessage />}
			/>
		</div>
	);
};
