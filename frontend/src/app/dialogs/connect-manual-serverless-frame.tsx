import type { Rivet } from "@rivetkit/engine-api-full";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import type { DialogContentProps } from "@/components";
import { type Region, useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { queryClient } from "@/queries/global";
import { EnvVariables } from "../env-variables";
import { StepperForm } from "../forms/stepper-form";
import { useSelectedDatacenter } from "./connect-manual-serverfull-frame";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: z.object({
			runnerName: z.string().min(1, "Runner name is required"),
			datacenters: z
				.record(z.boolean())
				.refine(
					(data) => Object.values(data).some(Boolean),
					"At least one datacenter must be selected",
				),
			headers: z.array(z.tuple([z.string(), z.string()])).default([]),
			slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
			maxRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
			requestLifespan: z.coerce.number().min(0, "Must be 0 or greater"),
		}),
	},
	{
		id: "step-2",
		title: "Deploy",
		assist: false,
		schema: z.object({}),
		next: "Next",
	},
	{
		id: "step-3",
		title: "Confirm Connection",
		assist: false,
		schema: z.object({
			endpoint: z
				.string()
				.nonempty("Endpoint is required")
				.url("Please enter a valid URL"),
			success: z.boolean().refine((v) => v === true, {
				message: "Runner must be connected to proceed",
			}),
		}),
		next: "Add",
	},
);

interface ConnectManualServerlessFrameContentProps extends DialogContentProps {}

export default function ConnectManualServerlessFrameContent({
	onClose,
}: ConnectManualServerlessFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().regionsQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().regionsQueryOptions(),
	);

	return <FormStepper onClose={onClose} datacenters={datacenters} />;
}

function FormStepper({
	onClose,
	datacenters,
}: {
	onClose?: () => void;
	datacenters: Region[];
}) {
	const provider = useEngineCompatDataProvider();

	const { data } = useSuspenseInfiniteQuery({
		...provider.runnerConfigsQueryOptions(),
	});

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			confetti({
				angle: 60,
				spread: 55,
				origin: { x: 0 },
			});
			confetti({
				angle: 120,
				spread: 55,
				origin: { x: 1 },
			});

			await queryClient.invalidateQueries(
				provider.runnerConfigsQueryOptions(),
			);
			onClose?.();
		},
	});
	return (
		<StepperForm
			{...stepper}
			onSubmit={async ({ values }) => {
				let existing: Record<string, Rivet.RunnerConfig> = {};
				try {
					const runnerConfig = await queryClient.fetchQuery(
						provider.runnerConfigQueryOptions({
							name: values.runnerName,
						}),
					);
					existing = runnerConfig?.datacenters || {};
				} catch {
					existing = {};
				}

				const selectedDatacenters = Object.entries(values.datacenters)
					.filter(([, selected]) => selected)
					.map(([id]) => id);

				const config = {
					serverless: {
						url: values.endpoint,
						maxRunners: values.maxRunners,
						slotsPerRunner: values.slotsPerRunner,
						runnersMargin: values.runnerMargin,
						requestLifespan: values.requestLifespan,
						headers: Object.fromEntries(
							values.headers.map(([key, value]) => [key, value]),
						),
					},
					metadata: {
						provider: "custom",
					},
				};

				const payload = {
					...existing,
					...Object.fromEntries(
						selectedDatacenters.map((dc) => [dc, config]),
					),
				};

				await mutateAsync({
					name: values.runnerName,
					config: payload,
				});
			}}
			defaultValues={{
				runnerName: "default",
				slotsPerRunner: 1,
				maxRunners: 10000,
				minRunners: 1,
				runnerMargin: 0,
				headers: [],
				success: false,
				requestLifespan: 900,
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.id, true]),
				),
			}}
			content={{
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 />,
				"step-3": () => <Step3 />,
			}}
		/>
	);
}

function Step1() {
	return (
		<div className="space-y-4">
			<ConnectServerlessForm.RunnerName />
			<ConnectServerlessForm.Datacenters />
			<ConnectServerlessForm.Headers />
			<ConnectServerlessForm.SlotsPerRunner />
			<ConnectServerlessForm.MaxRunners />
			<ConnectServerlessForm.MinRunners />
			<ConnectServerlessForm.RunnerMargin />
			<ConnectServerlessForm.RequestLifespan />
		</div>
	);
}

function Step2() {
	return (
		<>
			<p>Set the following environment variables.</p>
			<EnvVariables
				endpoint={useSelectedDatacenter()}
				runnerName={useWatch({ name: "runnerName" })}
			/>
		</>
	);
}

function Step3() {
	return (
		<>
			<ConnectServerlessForm.Endpoint placeholder="https://your-serverless-endpoint.com/api/rivet" />
			<ConnectServerlessForm.ConnectionCheck provider="Your serverless provider" />
		</>
	);
}
