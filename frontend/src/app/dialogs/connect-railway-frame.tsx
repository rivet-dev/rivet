import { faRailway, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import z from "zod";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	Button,
	CopyButton,
	type DialogContentProps,
	DiscreteInput,
	Frame,
	Label,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { engineEnv } from "@/lib/env";
import { queryClient } from "@/queries/global";
import { useRailwayTemplateLink } from "@/utils/use-railway-template-link";
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
		title: "Deploy to Railway",
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

interface ConnectRailwayFrameContentProps extends DialogContentProps {}

export default function ConnectRailwayFrameContent({
	onClose,
}: ConnectRailwayFrameContentProps) {
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
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faRailway} className="ml-0.5" /> Railway
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<FormStepper
					onClose={onClose}
					defaultDatacenter={prefferedRegionForRailway}
				/>
			</Frame.Content>
		</>
	);
}

function FormStepper({
	onClose,
	defaultDatacenter,
}: {
	onClose?: () => void;
	defaultDatacenter: string;
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
			onSubmit={async ({ values }) => {
				await mutateAsync({
					name: values.runnerName,
					config: {
						[values.datacenter]: {
							normal: {},
							metadata: { provider: "railway" },
						},
					},
				});
			}}
			defaultValues={{
				runnerName: "default",
				success: true,
				datacenter: defaultDatacenter,
			}}
			content={{
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 />,
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
					<RivetTokenEnv />
					<RivetNamespaceEnv />
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
				We're going to help you deploy a RivetKit project to Railway and
				connect it to Rivet.
			</div>
			<Accordion type="single" collapsible>
				<AccordionItem value="item-1">
					<AccordionTrigger className="text-sm">
						Advanced
					</AccordionTrigger>
					<AccordionContent className="space-y-4 px-1 pt-2">
						<ConnectRailwayForm.RunnerName />
						<ConnectRailwayForm.Datacenter />
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</>
	);
}

function Step2() {
	return (
		<>
			<p>Deploy any RivetKit app to Railway.</p>
			<p>Or use our Railway template to get started quickly.</p>
			<DeployToRailwayButton />
			<p>
				Set the following environment variables in your Railway project
				settings.
			</p>
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
					show
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

function DeployToRailwayButton() {
	const runnerName = useWatch({ name: "runnerName" });

	const url = useRailwayTemplateLink({
		runnerName: runnerName || "default",
		datacenter: useWatch({ name: "datacenter" }) || "auto",
	});

	return (
		<a
			href={url}
			target="_blank"
			rel="noreferrer"
			className="inline-block h-10"
		>
			<img
				height={40}
				src="https://railway.com/button.svg"
				alt="Deploy to Railway"
			/>
		</a>
	);
}

export const useSelectedDatacenter = () => {
	const datacenter = useWatch({ name: "datacenter" });

	const { data } = useQuery(
		useEngineCompatDataProvider().regionQueryOptions(datacenter || "auto"),
	);

	return data?.url || engineEnv().VITE_APP_API_URL;
};
