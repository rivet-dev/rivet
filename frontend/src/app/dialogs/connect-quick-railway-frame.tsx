import { faRailway, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import z from "zod";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	type DialogContentProps,
	ExternalLinkCard,
	Frame,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { queryClient } from "@/queries/global";
import { useRailwayTemplateLink } from "@/utils/use-railway-template-link";
import { StepperForm } from "../forms/stepper-form";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Deploy to Railway",
		assist: false,
		next: "Next",
		schema: z.object({
			runnerName: z.string().min(1, "Runner name is required"),
			datacenter: z.string().min(1, "Please select a region"),
		}),
	},
	{
		id: "step-2",
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

interface ConnectQuickRailwayFrameContentProps extends DialogContentProps {}

export default function ConnectQuickRailwayFrameContent({
	onClose,
}: ConnectQuickRailwayFrameContentProps) {
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
				"step-1": () => <Step1 datacenter={defaultDatacenter} />,
				"step-2": () => <Step2 />,
			}}
		/>
	);
}

function Step1({ datacenter }: { datacenter: string }) {
	return (
		<>
			<div className="space-y-4">
				<p>Deploy the Rivet Railway template to get started quickly.</p>
				<DeployToRailwayButton datacenter={datacenter} />
			</div>
			<Accordion type="single" collapsible>
				<AccordionItem value="item-1">
					<AccordionTrigger className="text-sm">
						Advanced Options
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

function DeployToRailwayButton({ datacenter }: { datacenter: string }) {
	const runnerName = "default";
	const url = useRailwayTemplateLink({
		runnerName,
		datacenter,
	});

	return (
		<ExternalLinkCard
			href={url}
			icon={faRailway}
			title="Deploy Template to Railway"
		/>
	);
}

function Step2() {
	return <ConnectRailwayForm.ConnectionCheck provider="railway" />;
}
