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
import {
	Button,
	CopyButton,
	type DialogContentProps,
	DiscreteInput,
	Label,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { engineEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
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
		...useEngineCompatDataProvider().regionsQueryOptions(),
		pages: Infinity,
	});
	const { data } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().regionsQueryOptions(),
	);

	const prefferedRegionForRailway =
		data.find((region) => region.name.toLowerCase().includes("us-west"))
			?.id ||
		data.find((region) => region.name.toLowerCase().includes("us-east"))
			?.id ||
		data.find((region) => region.name.toLowerCase().includes("ore"))?.id ||
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

export function EnvVariablesStep() {
	return (
		<>
			<div>
				<div
					className="gap-1 items-center grid grid-cols-2"
					data-env-variables
				>
					<Label
						asChild
						className="text-muted-foreground text-xs mb-1"
					>
						<p>Key</p>
					</Label>
					<Label
						asChild
						className="text-muted-foreground text-xs mb-1"
					>
						<p>Value</p>
					</Label>
					<RivetEndpointEnv />
					<RivetNamespaceEnv />
					<RivetTokenEnv />
					<RivetRunnerEnv />
				</div>
				<div className="mt-2 flex justify-end">
					<CopyButton
						value={() => {
							const inputs =
								document.querySelectorAll<HTMLInputElement>(
									"[data-env-variables] input",
								);
							return Array.from(inputs)
								.reduce((acc, input, index) => {
									if (index % 2 === 0) {
										acc.push(
											`${input.value}=${inputs[index + 1]?.value}`,
										);
									}
									return acc;
								}, [] as string[])
								.join("\n");
						}}
					>
						<Button size="sm" variant="outline">
							Copy all raw
						</Button>
					</CopyButton>
				</div>
			</div>
		</>
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

			<EnvVariablesStep />
		</>
	);
}

function Step3() {
	return <ConnectRailwayForm.ConnectionCheck provider="railway" />;
}

function RivetRunnerEnv() {
	const runnerName = useWatch({ name: "runnerName" });
	if (runnerName === "default") return null;

	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_RUNNER"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={runnerName || "default"}
				show
			/>
		</>
	);
}

function RivetTokenEnv() {
	const { data, isLoading } = useQuery(
		useEngineCompatDataProvider().engineAdminTokenQueryOptions(),
	);
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_TOKEN"
				show
			/>
			{isLoading ? (
				<Skeleton className="w-full h-10" />
			) : (
				<DiscreteInput
					aria-label="environment variable value"
					value={(data as string) || ""}
				/>
			)}
		</>
	);
}

function RivetEndpointEnv() {
	const url = useSelectedDatacenter();
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_ENDPOINT"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={url}
				show
			/>
		</>
	);
}

function RivetNamespaceEnv() {
	const dataProvider = useEngineCompatDataProvider();
	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_NAMESPACE"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={dataProvider.engineNamespace || ""}
				show
			/>
		</>
	);
}

const useSelectedDatacenter = () => {
	const datacenter = useWatch({ name: "datacenter" });

	const { data } = useQuery(
		useEngineCompatDataProvider().regionQueryOptions(datacenter || "auto"),
	);

	return data?.url || engineEnv().VITE_APP_API_URL;
};
