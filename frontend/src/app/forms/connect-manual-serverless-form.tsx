import { faTrash, Icon } from "@rivet-gg/icons";
import type { Provider } from "@rivetkit/shared-data";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useFieldArray, useFormContext } from "react-hook-form";
import z from "zod";
import {
	endpointSchema,
	ServerlessConnectionCheck,
} from "@/app/serverless-connection-check";
import {
	Button,
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
import { RegionSelect } from "@/components/actors/region-select";
import { defineStepper } from "@/components/ui/stepper";

export { endpointSchema };

export const configurationSchema = z.object({
	runnerName: z.string().min(1, "Runner name is required"),
	datacenters: z
		.record(z.string(), z.boolean())
		.refine(
			(data) => Object.values(data).some(Boolean),
			"At least one datacenter must be selected",
		),
	headers: z.array(z.tuple([z.string(), z.string()])).default([]),
	slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
	maxRunners: z.coerce.number().min(1, "Must be at least 1"),
	minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
	runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
	requestLifespan: z.coerce.number().min(0, "Must be 0 or greater"),
});

export const deploymentSchema = z.object({
	success: z.boolean().refine((val) => val, "Connection failed"),
	endpoint: endpointSchema,
});

export const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: configurationSchema,
	},
	{
		id: "step-2",
		title: "Edit vercel.json",
		assist: false,
		next: "Next",
		schema: z.object({}),
	},
	{
		id: "step-3",
		title: "Deploy to Vercel",
		assist: true,
		next: "Done",
		schema: deploymentSchema,
	},
);

export const RunnerName = function RunnerName() {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="runnerName"
			render={({ field }) => (
				<FormItem>
					<FormLabel className="col-span-1">Runner Name</FormLabel>
					<FormControl className="row-start-2">
						<Input type="text" {...field} />
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Datacenters = function Datacenter() {
	const { control } = useFormContext();
	const { data: datacenterCount } = useInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		select: (data) =>
			data.pages.reduce(
				(prev, current) => prev + current.datacenters.length,
				0,
			),
	});

	return (
		<div className="space-y-2">
			<Label>Datacenters</Label>
			<FormDescription>
				Rivet datacenters that actors can be created in.
			</FormDescription>

			<div className="space-y-4">
				<FormField
					control={control}
					name="datacenters"
					render={({ field }) => (
						<RegionSelect
							showAuto={false}
							value={Object.keys(field.value || {}).filter(
								(key) => field.value[key],
							)}
							renderCurrentOptions={(options) => {
								if (options.length === datacenterCount) {
									return <span>Global</span>;
								}
								return options.map((option) => (
									<span key={option.value} className="mr-2">
										<ActorRegion
											regionId={option.value}
											showLabel
										/>
									</span>
								));
							}}
							onValueChange={(value) => {
								field.onChange(
									value.reduce(
										(acc, key) => {
											acc[key] = true;
											return acc;
										},
										{} as Record<string, boolean>,
									),
								);
							}}
							multiple
						/>
					)}
				/>
			</div>
		</div>
	);
};

export const MinRunners = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="minRunners"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Min Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="number"
							{...field}
							value={field.value || ""}
							min={0}
						/>
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

export const MaxRunners = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="maxRunners"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Max Runners</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="number"
							{...field}
							value={field.value || ""}
							min={0}
						/>
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

export const SlotsPerRunner = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="slotsPerRunner"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Slots Per Runner
					</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="number"
							{...field}
							value={field.value || ""}
							min={0}
						/>
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

export const RunnerMargin = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="runnerMargin"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Runner Margin</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="number"
							{...field}
							value={field.value}
							min={0}
						/>
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

export const RequestLifespan = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="requestLifespan"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">
						Request Lifespan (seconds)
					</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="number"
							{...field}
							value={field.value}
							min={0}
						/>
					</FormControl>
					<FormDescription className="col-span-1">
						The maximum duration (in seconds) that a request can run
						before being terminated.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export const Headers = function Headers() {
	const { control, setValue, watch } = useFormContext();
	const { fields, append, remove } = useFieldArray({
		name: "headers",
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
							value={{ name: `headers.${index}.0` }}
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
										value={watch(`headers.${index}.0`)}
										onChange={(e) => {
											setValue(
												`headers.${index}.0`,
												e.target.value,
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
							value={{ name: `headers.${index}.1` }}
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
										value={watch(`headers.${index}.1`)}
										onChange={(e) => {
											setValue(
												`headers.${index}.1`,
												e.target.value,
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
				onClick={() => append([["", ""]])}
			>
				Add a header
			</Button>
		</div>
	);
};

export const Endpoint = ({
	className,
	placeholder,
}: {
	className?: string;
	placeholder?: string;
}) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="endpoint"
			render={({ field }) => {
				return (
					<FormItem className={className}>
						<FormLabel className="col-span-1">Endpoint</FormLabel>
						<FormControl className="row-start-2">
							<Input
								type="text"
								placeholder={
									placeholder ||
									"https://my-rivet-app.vercel.app/api/rivet"
								}
								{...field}
								value={field.value || ""}
								onChange={(e) => {
									const value = e.target.value;
									field.onChange(value);
								}}
								onPaste={(e) => {
									const pastedText = e.clipboardData
										.getData("text")
										.trim();
									// Auto-add https:// if pasted URL doesn't have a protocol
									if (
										pastedText &&
										!pastedText.match(/^https?:\/\//i)
									) {
										e.preventDefault();
										field.onChange(`https://${pastedText}`);
									}
								}}
							/>
						</FormControl>
						<FormMessage className="col-span-1" />
					</FormItem>
				);
			}}
		/>
	);
};

export function ConnectionCheck({ provider }: { provider: Provider }) {
	return (
		<ServerlessConnectionCheck provider={provider} pollIntervalMs={3_000} />
	);
}
