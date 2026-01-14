import { faRailway, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import z from "zod";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import { type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { queryClient } from "@/queries/global";
import { EnvVariables } from "../env-variables";
import {
	configurationSchema,
	deploymentSchema,
} from "../forms/connect-manual-serverless-form";
import { StepperForm } from "../forms/stepper-form";
import { useEndpoint } from "./connect-manual-serverfull-frame";
import {
	buildServerlessConfig,
	ConfigurationAccordion,
} from "./connect-manual-serverless-frame";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: z.object({}),
	},
	{
		id: "step-2",
		title: "Deploy to Railway",
		assist: false,
		schema: z.object({
			...configurationSchema.shape,
			...deploymentSchema.shape,
		}),
		next: "Done",
	},
);

interface ConnectRailwayFrameContentProps extends DialogContentProps {}

export default function ConnectRailwayFrameContent({
	onClose,
}: ConnectRailwayFrameContentProps) {
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
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
	);
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
				"step-2": () => <StepDeploy />,
			}}
		/>
	);
}

function Step1() {
	return (
		<>
			<p>
				If you have not deployed a project, see the{" "}
				<a
					href="https://www.rivet.dev/docs/connect/railway"
					target="_blank"
					rel="noopener noreferrer"
					className="underline hover:text-foreground"
				>
					Railway quickstart guide
				</a>
				.
			</p>
			<p>
				Set these variables in Settings &gt; Variables in the Railway
				dashboard.
			</p>
			<EnvVariables
				endpoint={useEndpoint()}
				runnerName={useWatch({ name: "runnerName" })}
			/>
		</>
	);
}

function StepDeploy() {
	return (
		<>
			<p>
				Deploy your code to Railway and paste your deployment's
				endpoint:
			</p>
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
