import { useFormContext } from "react-hook-form";
import z from "zod";
import * as ConnectManualServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	CodeFrame,
	CodePreview,
	FormControl,
	FormDescription,
	FormField,
	FormItem,
	FormLabel,
	FormMessage,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";

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
			minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
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
		assist: true,
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

export const RunnerName = ConnectManualServerlessForm.RunnerName;

export const Datacenters = ConnectManualServerlessForm.Datacenters;

export const MinRunners = ConnectManualServerlessForm.MinRunners;

export const MaxRunners = ConnectManualServerlessForm.MaxRunners;

export const SlotsPerRunner = ConnectManualServerlessForm.SlotsPerRunner;

export const RunnerMargin = ConnectManualServerlessForm.RunnerMargin;

export const Headers = ConnectManualServerlessForm.Headers;

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

export const Endpoint = ConnectManualServerlessForm.Endpoint;

export const ConnectionCheck = ConnectManualServerlessForm.ConnectionCheck;
