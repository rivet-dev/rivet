import {
	faCheck,
	faSpinnerThird,
	faTrash,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect } from "react";
import {
	useController,
	useFieldArray,
	useForm,
	useFormContext,
	useWatch,
} from "react-hook-form";
import { match, P } from "ts-pattern";
import { useDebounceValue } from "usehooks-ts";
import z from "zod";
import {
	Button,
	Checkbox,
	CodeFrame,
	CodePreview,
	cn,
	FormControl,
	FormDescription,
	FormField,
	FormFieldContext,
	FormItem,
	FormLabel,
	FormMessage,
	Input,
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { VisibilitySensor } from "@/components/visibility-sensor";

const endpointSchema = z
	.string()
	.nonempty("Endpoint is required")
	.url("Please enter a valid URL")
	.endsWith("/api/rivet", "Endpoint must end with /api/rivet");

export const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: z.object({
			plan: z.string().min(1, "Please select a Vercel plan"),
			runnerName: z.string().min(1, "Runner name is required"),
			datacenters: z
				.record(z.boolean())
				.refine(
					(data) => Object.values(data).some(Boolean),
					"At least one datacenter must be selected",
				),
			headers: z.array(z.tuple([z.string(), z.string()])).default([]),
			slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
			maxRunners: z.coerce.number().min(1, "Must be at least 1"),
			runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
		}),
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
		assist: false,
		next: "Done",
		schema: z.object({
			success: z.boolean().refine((val) => val, "Connection failed"),
			endpoint: endpointSchema,
		}),
	},
);

export const Plan = ({ className }: { className?: string }) => {
	const { control } = useFormContext();
	return (
		<FormField
			control={control}
			name="plan"
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Vercel Plan</FormLabel>
					<FormControl className="row-start-2">
						<Select
							onValueChange={field.onChange}
							value={field.value}
						>
							<SelectTrigger>
								<SelectValue placeholder="Select your Vercel plan..." />
							</SelectTrigger>
							<SelectContent>
								<SelectItem value="hobby">Hobby</SelectItem>
								<SelectItem value="pro">Pro</SelectItem>
								<SelectItem value="enterprise">
									Enterprise
								</SelectItem>
							</SelectContent>
						</Select>
					</FormControl>
					<FormDescription className="col-span-1">
						Your Vercel plan determines the configuration required
						to properly connect Rivet to Vercel Functions.
					</FormDescription>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

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
	const { data, hasNextPage, fetchNextPage } = useInfiniteQuery(
		useEngineCompatDataProvider().regionsQueryOptions(),
	);

	return (
		<div className="space-y-2">
			<Label>Datacenters</Label>
			<FormDescription>
				Rivet datacenters that actors can be created in.
			</FormDescription>

			<div className="space-y-4">
				{data?.map((region) => (
					<FormField
						key={region.id}
						control={control}
						name={`datacenters.${region.id}`}
						render={({ field }) => (
							<div className="flex items-start gap-3">
								<Checkbox
									id={region.id}
									checked={field.value}
									name={field.name}
									onCheckedChange={field.onChange}
								/>
								<div className="grid gap-2">
									<Label htmlFor={region.id}>
										{region.name}
									</Label>
								</div>
							</div>
						)}
					/>
				))}
				{hasNextPage ? (
					<VisibilitySensor onChange={fetchNextPage} />
				) : null}
			</div>
		</div>
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
							min={1}
							{...field}
							value={field.value || ""}
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
							min={1}
							{...field}
							value={field.value || ""}
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
						<Input type="number" {...field} value={field.value} />
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
										value={field[0]}
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
										value={field[1]}
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

export const PLAN_TO_MAX_DURATION: Record<string, number> = {
	hobby: 60,
	pro: 300,
	enterprise: 900,
};

const code = ({ plan }: { plan: string }) =>
	`{
	"$schema": "https://openapi.vercel.sh/vercel.json",
	"fluid": false,	// [!code highlight]
	"functions": {
		"app/api/rivet/**": {
			"maxDuration": ${PLAN_TO_MAX_DURATION[plan] || 60},	// [!code highlight]
		},
	}
}`;

export const Json = ({ plan }: { plan: string }) => {
	return (
		<div className="space-y-2 mt-2">
			<CodeFrame
				language="json"
				title="vercel.json"
				code={() =>
					code({ plan }).replaceAll("	// [!code highlight]", "")
				}
			>
				<CodePreview
					className="w-full min-w-0"
					language="json"
					code={code({ plan })}
				/>
			</CodeFrame>
			<p className="col-span-1 text-sm text-muted-foreground">
				<b>Max Duration</b> - The maximum execution time of your
				serverless functions.
				<br />
				<b>Disable Fluid Compute</b> - Rivet has its own intelligent
				load balancing mechanism.
			</p>
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
			render={({ field }) => (
				<FormItem className={className}>
					<FormLabel className="col-span-1">Endpoint</FormLabel>
					<FormControl className="row-start-2">
						<Input
							type="url"
							placeholder={
								placeholder ||
								"https://my-rivet-app.vercel.app/api/rivet"
							}
							{...field}
						/>
					</FormControl>
					<FormMessage className="col-span-1" />
				</FormItem>
			)}
		/>
	);
};

export function ConnectionCheck() {
	const dataProvider = useEngineCompatDataProvider();

	const endpoint: string = useWatch({ name: "endpoint" });
	const headers: [string, string][] = useWatch({ name: "headers" });

	const enabled = !!endpoint && endpointSchema.safeParse(endpoint).success;

	const [debounced] = useDebounceValue(endpoint, 300);

	const { isSuccess, data, isError, isRefetchError, isLoadingError, error } =
		useQuery({
			...dataProvider.runnerHealthCheckQueryOptions({
				runnerUrl: debounced,
				headers: Object.fromEntries(
					headers.filter(([k, v]) => k && v).map(([k, v]) => [k, v]),
				),
			}),
			enabled,
			retry: 0,
			refetchInterval: 3_000,
		});

	const {
		field: { onChange },
	} = useController({ name: "success" });

	useEffect(() => {
		onChange(isSuccess);
	}, [isSuccess]);

	return (
		<AnimatePresence>
			{enabled ? (
				<motion.div
					layoutId="msg"
					className={cn(
						"text-center text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
						isSuccess && "text-primary-foreground",
						isError && "text-destructive-foreground",
					)}
					initial={{ height: 0, opacity: 0.5 }}
					animate={{ height: "8rem", opacity: 1 }}
				>
					{isSuccess ? (
						<>
							<Icon
								icon={faCheck}
								className="mr-1.5 text-primary"
							/>{" "}
							Vercel is running with RivetKit{" "}
							{(data as any)?.version}
						</>
					) : isError || isRefetchError || isLoadingError ? (
						<div className="flex flex-col items-center gap-2">
							<p className="flex items-center">
								<Icon
									icon={faTriangleExclamation}
									className="mr-1.5 text-destructive"
								/>{" "}
								Health check failed, verify the endpoint is
								correct.
							</p>
							{isRivetHealthCheckFailureResponse(error) ? (
								<HealthCheckFailure error={error} />
							) : null}
							<p>
								Endpoint{" "}
								<a
									className="underline"
									href={endpoint}
									target="_blank"
									rel="noopener noreferrer"
								>
									{endpoint}
								</a>
							</p>
						</div>
					) : (
						<div className="flex flex-col items-center gap-2">
							<div className="flex items-center">
								<Icon
									icon={faSpinnerThird}
									className="mr-1.5 animate-spin"
								/>{" "}
								Waiting for Runner to connect...
							</div>
						</div>
					)}
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}

function isRivetHealthCheckFailureResponse(
	error: any,
): error is Rivet.RunnerConfigsServerlessHealthCheckResponseFailure["failure"] {
	return error && "error" in error;
}

function HealthCheckFailure({
	error,
}: {
	error: Rivet.RunnerConfigsServerlessHealthCheckResponseFailure["failure"];
}) {
	if (!("error" in error)) {
		return null;
	}
	if (!error.error) {
		return null;
	}

	return match(error.error)
		.with({ nonSuccessStatus: P.any }, (e) => {
			return (
				<p>
					Health check failed with status{" "}
					{e.nonSuccessStatus.statusCode}
				</p>
			);
		})
		.with({ invalidRequest: P.any }, (e) => {
			return <p>Health check failed because the request was invalid.</p>;
		})
		.with({ invalidResponseJson: P.any }, (e) => {
			return (
				<p>
					Health check failed because the response was not valid JSON.
				</p>
			);
		})
		.with({ requestFailed: P.any }, (e) => {
			return (
				<p>
					Health check failed because the request could not be
					completed.
				</p>
			);
		})
		.with({ requestTimedOut: P.any }, (e) => {
			return <p>Health check failed because the request timed out.</p>;
		})
		.with({ invalidResponseSchema: P.any }, (e) => {
			return (
				<p>Health check failed because the response was not valid.</p>
			);
		})
		.exhaustive();
}
