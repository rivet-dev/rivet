import type { Rivet } from "@rivetkit/engine-api-full";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import { match } from "ts-pattern";
import z from "zod";
import * as ConnectRailwayForm from "@/app/forms/connect-manual-serverfull-form";
import type { DialogContentProps } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { engineEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
import { EnvVariables } from "../env-variables";
import { StepperForm } from "../forms/stepper-form";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
		assist: false,
		next: "Next",
		schema: z.object({
			runnerName: z.string().min(1, "Runner name is required"),
			datacenter: z.string().min(1, "Please select a region"),
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
		title: "Wait for the Runner to connect",
		assist: true,
		schema: z.object({
			success: z.boolean().refine((v) => v === true, {
				message: "Runner must be connected to proceed",
			}),
		}),
		next: "Add",
	},
);

interface ConnectManualServerlfullFrameContentProps extends DialogContentProps {
	provider: string;
}

export default function ConnectManualServerlfullFrameContent({
	onClose,
	provider,
}: ConnectManualServerlfullFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});
	const { data } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
	);

	const prefferedRegionForRailway =
		data.find((region) => region.name.toLowerCase().includes("us-west"))
			?.name ||
		data.find((region) => region.name.toLowerCase().includes("us-east"))
			?.name ||
		data.find((region) => region.name.toLowerCase().includes("ore"))
			?.name ||
		"auto";

	return (
		<FormStepper
			onClose={onClose}
			provider={provider}
			defaultDatacenter={prefferedRegionForRailway}
		/>
	);
}

function FormStepper({
	onClose,
	defaultDatacenter,
	provider,
}: {
	onClose?: () => void;
	provider: string;
	defaultDatacenter: string;
}) {
	const dataProvider = useEngineCompatDataProvider();

	const { data } = useSuspenseInfiniteQuery({
		...dataProvider.runnerConfigsQueryOptions(),
	});

	const { mutateAsync } = useMutation({
		...dataProvider.upsertRunnerConfigMutationOptions(),
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
				dataProvider.runnerConfigsQueryOptions(),
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
						dataProvider.runnerConfigQueryOptions({
							name: values.runnerName,
						}),
					);
					existing = runnerConfig?.datacenters || {};
				} catch {
					existing = {};
				}

				await mutateAsync({
					name: values.runnerName,
					config: {
						...existing,
						[values.datacenter]: {
							normal: {},
							metadata: { provider },
						},
					},
				});
			}}
			defaultValues={{
				runnerName: "default",
				success: false,
				datacenter: defaultDatacenter,
			}}
			content={{
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 provider={provider} />,
				"step-3": () => <Step3 />,
			}}
		/>
	);
}

function Step1() {
	return (
		<>
			<div>
				We're going to help you deploy a RivetKit project to your cloud
				provider of choice.
			</div>
			<ConnectRailwayForm.RunnerName />
			<ConnectRailwayForm.Datacenter />
		</>
	);
}

function Step2({ provider }: { provider: string }) {
	return (
		<>
			{match(provider)
				.with("aws", () => (
					<p>
						<a
							href="https://www.rivet.dev/docs/connect/aws-ecs/"
							className="underline"
							target="_blank"
							rel="noopener"
						>
							Follow the integration guide here
						</a>
						, and make sure to set the following environment
						variables:
					</p>
				))
				.with("hetzner", () => (
					<p>
						<a
							href="https://www.rivet.dev/docs/connect/hetzner/"
							className="underline"
							target="_blank"
							rel="noopener"
						>
							Follow the integration guide here
						</a>
						, and make sure to set the following environment
						variables:
					</p>
				))
				.with("gcp", () => (
					<p>
						<a
							href="https://www.rivet.dev/docs/connect/gcp-cloud-run/"
							className="underline"
							target="_blank"
							rel="noopener"
						>
							Follow the integration guide here
						</a>
						, and make sure to set the following environment
						variables:
					</p>
				))
				.otherwise(() => (
					<p>Set the following environment variables.</p>
				))}

			<EnvVariables
				kind="serverfull"
				endpoint={useSelectedDatacenter()}
				runnerName={useWatch({ name: "runnerName" })}
			/>
		</>
	);
}

function Step3() {
	return <ConnectRailwayForm.ConnectionCheck provider="railway" />;
}

export const useSelectedDatacenter = () => {
	const datacenter = useWatch({ name: "datacenter" });

	const { data } = useQuery(
		useEngineCompatDataProvider().datacenterQueryOptions(
			datacenter || "auto",
		),
	);

	return data?.url || engineEnv().VITE_APP_API_URL;
};
