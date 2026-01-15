import { faRailway, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import { type DialogContentProps, ExternalLinkCard, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { queryClient } from "@/queries/global";
import { useRailwayTemplateLink } from "@/utils/use-railway-template-link";
import {
	configurationSchema,
	deploymentSchema,
} from "../forms/connect-manual-serverless-form";
import { StepperForm } from "../forms/stepper-form";
import {
	buildServerlessConfig,
	ConfigurationAccordion,
} from "./connect-manual-serverless-frame";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: true,
		next: "Next",
		schema: configurationSchema,
	},
	{
		id: "deploy",
		title: "Configure Railway endpoint",
		assist: true,
		next: "Done",
		schema: deploymentSchema,
	},
);

interface ConnectQuickRailwayFrameContentProps extends DialogContentProps {}

export default function ConnectQuickRailwayFrameContent({
	onClose,
}: ConnectQuickRailwayFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faRailway} className="ml-0.5" /> Railway
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<FormStepper onClose={onClose} />
			</Frame.Content>
		</>
	);
}

function FormStepper({ onClose }: { onClose?: () => void }) {
	const provider = useEngineCompatDataProvider();
	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
	);
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
			onSubmit={async ({ values }) => {
				const payload = await buildServerlessConfig(provider, values, {
					provider: "railway",
				});
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
				requestLifespan: 55,
				headers: [],
				success: false,
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.name, true]),
				),
			}}
			content={{
				"step-1": () => <Step1 />,
				deploy: () => <DeployStep />,
			}}
		/>
	);
}

function Step1() {
	return (
		<>
			<div className="space-y-4">
				<p>Deploy the Rivet Railway template to get started quickly.</p>
				<DeployToRailwayButton />
			</div>
		</>
	);
}

function DeployStep() {
	return (
		<>
			<p>Paste your deployment's endpoint below:</p>
			<div className="mt-2">
				<ConnectRailwayForm.Endpoint placeholder="https://my-rivet-app.up.railway.app" />

				<ConfigurationAccordion />
				<p className="text-muted-foreground text-sm">
					Need help deploying? See{" "}
					<a
						href="https://docs.railway.com/guides/deployments"
						target="_blank"
						rel="noreferrer"
						className="underline"
					>
						Railway's deployment documentation
					</a>
					.
				</p>
			</div>
			<ConnectRailwayForm.ConnectionCheck provider="Railway" />
		</>
	);
}

function DeployToRailwayButton() {
	const runnerName = useWatch({ name: "runnerName" });
	const url = useRailwayTemplateLink({
		runnerName,
	});

	return (
		<ExternalLinkCard
			href={url}
			icon={faRailway}
			title="Deploy Template to Railway"
		/>
	);
}
