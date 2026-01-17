import type { Rivet } from "@rivetkit/engine-api-full";
import type { Provider } from "@rivetkit/example-registry";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import type { ComponentProps, ReactNode } from "react";
import { useWatch } from "react-hook-form";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import { endpointSchema } from "@/app/forms/connect-manual-serverless-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	type DialogContentProps,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
import { EnvVariables } from "../env-variables";
import { StepperForm } from "../forms/stepper-form";
import { useEndpoint } from "./connect-manual-serverfull-frame";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: ConnectServerlessForm.configurationSchema,
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
		schema: ConnectServerlessForm.deploymentSchema,
		next: "Add",
	},
);

interface ConnectManualServerlessFrameContentProps extends DialogContentProps {
	provider: Provider;
}

export default function ConnectManualServerlessFrameContent({
	onClose,
	provider,
}: ConnectManualServerlessFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
	);

	return (
		<FormStepper
			onClose={onClose}
			datacenters={datacenters}
			provider={provider}
		/>
	);
}

function FormStepper({
	onClose,
	datacenters,
	provider,
}: {
	onClose?: () => void;
	datacenters: Rivet.Datacenter[];
	provider: Provider;
}) {
	const dataProvider = useEngineCompatDataProvider();

	const { mutateAsync } = useMutation({
		...dataProvider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			successfulBackendSetupEffect();

			await queryClient.invalidateQueries(
				dataProvider.runnerConfigsQueryOptions(),
			);
			onClose?.();
		},
	});
	return (
		<StepperForm
			{...stepper}
			onSubmit={async ({ values }) => {
				const payload = await buildServerlessConfig(
					dataProvider,
					values,
					{
						provider,
					},
				);

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
					datacenters.map((dc) => [dc.name, true]),
				),
			}}
			content={{
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 />,
				"step-3": () => <Step3 provider={provider} />,
			}}
		/>
	);
}

export const buildServerlessConfig = async (
	dataProvider: ReturnType<typeof useEngineCompatDataProvider>,
	values: z.infer<
		typeof ConnectServerlessForm.configurationSchema &
			typeof ConnectServerlessForm.deploymentSchema
	>,
	{ provider }: { provider?: string } = {},
): Promise<Record<string, Rivet.RunnerConfig>> => {
	let existing: Record<string, Rivet.RunnerConfig> = {};
	try {
		const runnerConfig = await queryClient.fetchQuery(
			dataProvider.runnerConfigQueryOptions({
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
			url: endpointSchema.parse(values.endpoint),
			maxRunners: values.maxRunners,
			slotsPerRunner: values.slotsPerRunner,
			runnersMargin: values.runnerMargin,
			requestLifespan: values.requestLifespan,
			headers: Object.fromEntries(
				values.headers.map(([key, value]) => [key, value]),
			),
		},
		metadata: {
			provider: provider || "custom",
		},
	};

	const payload = {
		...existing,
		...Object.fromEntries(selectedDatacenters.map((dc) => [dc, config])),
	};

	return payload;
};

function Step1() {
	return (
		<div className="space-y-4">
			<Configuration />
		</div>
	);
}

function Step2() {
	return (
		<>
			<p>Set the following environment variables.</p>
			<EnvVariables
				endpoint={useEndpoint()}
				runnerName={useWatch({ name: "runnerName" })}
			/>
		</>
	);
}

function Step3({ provider }: { provider: Provider }) {
	return (
		<>
			<ConnectServerlessForm.Endpoint placeholder="https://your-serverless-endpoint.com/api/rivet" />
			<ConnectServerlessForm.ConnectionCheck provider={provider} />
		</>
	);
}

export function Configuration({
	runnerName = true,
	datacenters = true,
	headers = true,
	slotsPerRunner = true,
	minRunners = true,
	maxRunners = true,
	runnerMargin = true,
	requestLifespan = true,
}: {
	runnerName?: boolean;
	datacenters?: boolean;
	headers?: boolean;
	slotsPerRunner?: boolean;
	minRunners?: boolean;
	maxRunners?: boolean;
	runnerMargin?: boolean;
	requestLifespan?: boolean;
}) {
	return (
		<>
			{runnerName && <ConnectServerlessForm.RunnerName />}
			{datacenters && <ConnectServerlessForm.Datacenters />}
			{headers && <ConnectServerlessForm.Headers />}
			{slotsPerRunner && <ConnectServerlessForm.SlotsPerRunner />}
			{minRunners && <ConnectServerlessForm.MinRunners />}
			{maxRunners && <ConnectServerlessForm.MaxRunners />}
			{runnerMargin && <ConnectServerlessForm.RunnerMargin />}
			{requestLifespan && <ConnectServerlessForm.RequestLifespan />}
		</>
	);
}

export function ConfigurationAccordion({
	prefixFields,
	...props
}: ComponentProps<typeof Configuration> & { prefixFields?: ReactNode }) {
	return (
		<Accordion type="single" collapsible>
			<AccordionItem value="item-1">
				<AccordionTrigger className="text-sm">
					Advanced
				</AccordionTrigger>
				<AccordionContent className="space-y-4 px-1 pt-2">
					{prefixFields}
					<Configuration {...props} />
				</AccordionContent>
			</AccordionItem>
		</Accordion>
	);
}
