import { faVercel, Icon } from "@rivet-gg/icons";
import type { Rivet } from "@rivetkit/engine-api-full";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useMemo } from "react";
import * as ConnectVercelForm from "@/app/forms/connect-quick-vercel-form";
import { type DialogContentProps, ExternalLinkCard, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";
import { useRivetDsn } from "../env-variables";
import { StepperForm } from "../forms/stepper-form";
import { useEndpoint } from "./connect-manual-serverfull-frame";
import {
	buildServerlessConfig,
	ConfigurationAccordion,
} from "./connect-manual-serverless-frame";
import { VERCEL_SERVERLESS_MAX_DURATION } from "./connect-vercel-frame";

const { stepper } = ConnectVercelForm;

interface ConnectQuickVercelFrameContentProps extends DialogContentProps {}

export default function ConnectQuickVercelFrameContent({
	onClose,
}: ConnectQuickVercelFrameContentProps) {
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
						Add <Icon icon={faVercel} className="ml-0.5" />
						Vercel
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
			showAllSteps
			initialStep="deploy"
			mode="all"
			content={{
				"initial-info": () => <StepInitialInfo />,
				deploy: () => <StepDeploy />,
			}}
			onSubmit={async ({ values }) => {
				const payload = await buildServerlessConfig(
					provider,
					{
						...values,
						requestLifespan: VERCEL_SERVERLESS_MAX_DURATION - 5,
					},
					{ provider: "vercel" },
				);

				await mutateAsync({
					name: values.runnerName,
					config: payload,
				});
			}}
			defaultValues={{
				runnerName: "default",
				slotsPerRunner: 1,
				minRunners: 1,
				maxRunners: 10_000,
				runnerMargin: 0,
				headers: [],
				success: false,
				plan: "hobby",
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.name, true]),
				),
			}}
		/>
	);
}

const useVercelTemplateLink = () => {
	const endpoint = useEndpoint();

	const dsn = useRivetDsn({ endpoint, kind: "serverless" });

	return useMemo(() => {
		const repositoryUrl = "https://github.com/rivet-dev/template-vercel";
		const env = ["RIVET_ENDPOINT", "NEXT_PUBLIC_RIVET_ENDPOINT"].join(",");
		const projectName = "rivetkit-vercel";
		const envDefaults = {
			RIVET_ENDPOINT: dsn,
			NEXT_PUBLIC_RIVET_ENDPOINT: dsn,
		};

		return `https://vercel.com/new/clone?repository-url=${encodeURIComponent(repositoryUrl)}&env=${env}&project-name=${projectName}&repository-name=${projectName}&envDefaults=${encodeURIComponent(JSON.stringify(envDefaults))}`;
	}, [dsn]);
};

function StepInitialInfo() {
	const vercelTemplateLink = useVercelTemplateLink();
	return (
		<>
			<div className="space-y-4">
				<p>Deploy the Rivet Vercel template to get started quickly.</p>
				<ExternalLinkCard
					href={vercelTemplateLink}
					icon={faVercel}
					title="Deploy Template to Vercel"
				/>
			</div>
			<div className="space-y-4">
				<p>Set the following environment variables:</p>
				<ConnectVercelForm.EnvVariables />
			</div>
		</>
	);
}

function StepDeploy() {
	return (
		<>
			<p>
				Deploy your code to Vercel and paste your deployment's endpoint:
			</p>
			<div className="mt-2">
				<ConnectVercelForm.Endpoint />
				<ConfigurationAccordion
					requestLifespan={false}
					prefixFields={<ConnectVercelForm.Plan />}
				/>
				<p className="text-muted-foreground text-sm">
					Need help deploying? See{" "}
					<a
						href="https://vercel.com/docs/deployments"
						target="_blank"
						rel="noreferrer"
						className="underline"
					>
						Vercel's deployment documentation
					</a>
					.
				</p>
			</div>

			<ConnectVercelForm.ConnectionCheck provider="Vercel" />
		</>
	);
}
