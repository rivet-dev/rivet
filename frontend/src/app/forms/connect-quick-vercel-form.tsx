import { useFormContext } from "react-hook-form";
import z from "zod";
import * as ConnectManualServerlessForm from "@/app/forms/connect-manual-serverless-form";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { defineStepper } from "@/components/ui/stepper";

const endpointSchema = z
	.string()
	.nonempty("Endpoint is required")
	.url("Please enter a valid URL")
	.endsWith("/api/rivet", "Endpoint must end with /api/rivet");

export const stepper = defineStepper(
	{
		id: "initial-info",
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
		id: "env-vars",
		title: "Configure Environment Variables",
		assist: false,
		next: "Next",
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
		}),
	},
);

export const PLAN_TO_MAX_DURATION = ConnectVercelForm.PLAN_TO_MAX_DURATION;

export const Plan = ConnectVercelForm.Plan;
export const RunnerName = ConnectVercelForm.RunnerName;

export const Datacenters = ConnectVercelForm.Datacenters;

export const MinRunners = ConnectVercelForm.MinRunners;

export const MaxRunners = ConnectVercelForm.MaxRunners;

export const SlotsPerRunner = ConnectVercelForm.SlotsPerRunner;

export const RunnerMargin = ConnectVercelForm.RunnerMargin;

export const Headers = ConnectVercelForm.Headers;

export const Endpoint = ConnectVercelForm.Endpoint;

export const ConnectionCheck = ConnectVercelForm.ConnectionCheck;
