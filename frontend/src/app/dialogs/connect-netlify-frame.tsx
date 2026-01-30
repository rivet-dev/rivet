import { faCloud, Icon } from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { useWatch } from "react-hook-form";
import * as ConnectNetlifyForm from "@/app/forms/connect-netlify-form";
import { type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
import { StepperForm } from "../forms/stepper-form";
import {
	buildServerlessConfig,
} from "./connect-manual-serverless-frame";

const { stepper } = ConnectNetlifyForm;

export const NETLIFY_SERVERLESS_MAX_DURATION = 10;

interface ConnectNetlifyFrameContentProps extends DialogContentProps {}

export default function ConnectNetlifyFrameContent({
	onClose,
}: ConnectNetlifyFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
	);

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faCloud} className="ml-0.5" />
						Netlify
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<FormStepper onClose={onClose} datacenters={datacenters} />
			</Frame.Content>
		</>
	);
}

function FormStepper({
	datacenters,
	onClose,
}: {
	onClose?: () => void;
	datacenters: Rivet.Datacenter[];
}) {
	const provider = useEngineCompatDataProvider();

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			successfulBackendSetupEffect();
			await queryClient.invalidateQueries(
				provider.runnerConfigsQueryOptions(),
			);
			onClose?.();
		},
	});
	return (
		<StepperForm
			{...stepper}
			content={{
				"api-route": () => <StepApiRoute />,
				frontend: () => <StepFrontend />,
				variables: () => (
					<>
						<p>
							Set these variables in Settings &gt; Environment
							Variables in the Netlify dashboard.
						</p>
						<ConnectNetlifyForm.EnvVariables />
					</>
				),
				deploy: () => <StepDeploy />,
			}}
			onSubmit={async ({ values }) => {
				const payload = await buildServerlessConfig(
					provider,
					{
						...values,
						requestLifespan: NETLIFY_SERVERLESS_MAX_DURATION - 5,
					},
					{ provider: "netlify" },
				);

				await mutateAsync({
					name: values.runnerName,
					config: payload,
				});
			}}
			defaultValues={{
				plan: "starter",
				runnerName: "default",
				slotsPerRunner: 1,
				minRunners: 1,
				maxRunners: 10_000,
				runnerMargin: 0,
				headers: [],
				success: false,
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.name, true]),
				),
			}}
		/>
	);
}

function StepApiRoute() {
	const plan = useWatch({ name: "plan" });
	return <ConnectNetlifyForm.IntegrationCode plan={plan || "starter"} />;
}

function StepFrontend() {
	return <ConnectNetlifyForm.FrontendIntegrationCode />;
}

function StepDeploy() {
	return (
		<>
			<ConnectNetlifyForm.Plan className="mb-4" />
			<ConnectNetlifyForm.RunnerName className="mb-4" />
			<ConnectNetlifyForm.Datacenters className="mb-4" />
			<ConnectNetlifyForm.MinRunners className="mb-4" />
			<ConnectNetlifyForm.MaxRunners className="mb-4" />
			<ConnectNetlifyForm.SlotsPerRunner className="mb-4" />
			<ConnectNetlifyForm.RunnerMargin className="mb-4" />
			<ConnectNetlifyForm.Headers className="mb-4" />
			<ConnectNetlifyForm.Endpoint className="mb-4" />
			<ConnectNetlifyForm.ConnectionCheck />
		</>
	);
}

export { ConnectNetlifyFrameContent as ConnectNetlifyFrame };