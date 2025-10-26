import { useFormContext } from "react-hook-form";
import z from "zod";
import * as ConnectManualServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	CodeFrame,
	CodeGroup,
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
	// {
	// 	id: "initial-info",
	// 	title: "Configure",
	// 	assist: false,
	// 	next: "Next",
	// 	schema: z.object({
	// 		plan: z.string().min(1, "Please select a Vercel plan"),
	// 		runnerName: z.string().min(1, "Runner name is required"),
	// 		datacenters: z
	// 			.record(z.boolean())
	// 			.refine(
	// 				(data) => Object.values(data).some(Boolean),
	// 				"At least one datacenter must be selected",
	// 			),
	// 		headers: z.array(z.tuple([z.string(), z.string()])).default([]),
	// 		slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
	// 		maxRunners: z.coerce.number().min(1, "Must be at least 1"),
	// 		minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
	// 		runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
	// 	}),
	// },
	{
		id: "api-route",
		title: "Add API Route",
		assist: false,
		schema: z.object({}),
		next: "Next",
	},
	{
		id: "frontend",
		title: "Connect Frontend",
		assist: false,
		next: "Next",
		optional: true,
		schema: z.object({}),
	},
	{
		id: "deploy",
		title: "Deploy to Vercel",
		assist: true,
		next: "Done",
		schema: z.object({
			success: z.boolean().refine((val) => val, "Connection failed"),
			endpoint: endpointSchema,
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
			plan: z.string().min(1, "Please select a Vercel plan"),
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

// export const PLAN_TO_MAX_DURATION: Record<string, number> = {
// 	hobby: 300,
// 	pro: 800,
// 	enterprise: 800,
// };
//
// const integrationCode = ({ plan }: { plan: string }) =>
// 	`import { toNextHandler } from "@rivetkit/next-js";
// import { registry } from "@/rivet/registry";
//
// export const maxDuration = ${PLAN_TO_MAX_DURATION[plan] || 60};	// [!code highlight]
//
// export const { GET, POST, PUT, PATCH, HEAD, OPTIONS } = toNextHandler(registry);`;

const integrationCode = ({ plan: _ }: { plan: string }) =>
	`import { toNextHandler } from "@rivetkit/next-js";
import { registry } from "@/rivet/registry";

export const maxDuration = 300;

export const { GET, POST, PUT, PATCH, HEAD, OPTIONS } = toNextHandler(registry);`;

export const IntegrationCode = ({ plan }: { plan: string }) => {
	return (
		<div className="space-y-4 mt-2">
			<p>
				If you have not created a project, see the{" "}
				<a
					href="https://www.rivet.dev/docs/actors/quickstart/next-js/"
					target="_blank"
					rel="noopener noreferrer"
					className="underline hover:text-foreground"
				>
					Next.js quickstart guide
				</a>
				.
			</p>
			<p>First, install the Rivet Next.js package:</p>
			<CodeGroup>
				<CodeFrame
					language="bash"
					title="npm"
					code={() => "npm install @rivetkit/next-js"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="npm install @rivetkit/next-js"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="pnpm"
					code={() => "pnpm add @rivetkit/next-js"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="pnpm add @rivetkit/next-js"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="yarn"
					code={() => "yarn add @rivetkit/next-js"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="yarn add @rivetkit/next-js"
					/>
				</CodeFrame>
				<CodeFrame
					language="bash"
					title="bun"
					code={() => "bun add @rivetkit/next-js"}
				>
					<CodePreview
						className="w-full min-w-0"
						language="bash"
						code="bun add @rivetkit/next-js"
					/>
				</CodeFrame>
			</CodeGroup>
			<p>Then, add your Rivet route handler for Rivet:</p>
			<CodeFrame
				language="typescript"
				title="src/app/api/rivet/[...all]/route.ts"
				code={() =>
					integrationCode({ plan }).replaceAll(
						"	// [!code highlight]",
						"",
					)
				}
			>
				<CodePreview
					className="w-full min-w-0"
					language="typescript"
					code={integrationCode({ plan })}
				/>
			</CodeFrame>
		</div>
	);
};

export const Endpoint = ConnectManualServerlessForm.Endpoint;

export const ConnectionCheck = ConnectManualServerlessForm.ConnectionCheck;

export const FrontendIntegrationCode = ({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) => {
	const clientCode = `"use client";
import { createRivetKit } from "@rivetkit/next-js/client";
import type { registry } from "@/rivet/registry";

export const { useActor } = createRivetKit<typeof registry>({
	endpoint: "${endpoint}",
	namespace: "${namespace}",
	token: "${token}",
});
`;

	return (
		<div className="space-y-4 mt-2">
			<p>Connect your Next.js frontend to Rivet:</p>
			<CodeFrame
				language="typescript"
				title="src/lib/rivet.ts"
				code={() => clientCode}
			>
				<CodePreview
					className="w-full min-w-0"
					language="typescript"
					code={clientCode}
				/>
			</CodeFrame>
			<p className="text-muted-foreground text-sm">
				This token is safe to publish on your frontend.
			</p>
		</div>
	);
};
