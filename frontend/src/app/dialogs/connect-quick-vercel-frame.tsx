import { faVercel, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import * as ConnectVercelForm from "@/app/forms/connect-quick-vercel-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	type DialogContentProps,
	ExternalLinkCard,
	Frame,
} from "@/components";
import { type Region, useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";
import { StepperForm } from "../forms/stepper-form";
import { EnvVariablesStep } from "./connect-railway-frame";
import { VERCEL_SERVERLESS_MAX_DURATION } from "./connect-vercel-frame";

const { stepper } = ConnectVercelForm;

interface ConnectQuickVercelFrameContentProps extends DialogContentProps {}

export default function ConnectQuickVercelFrameContent({
	onClose,
}: ConnectQuickVercelFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().regionsQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().regionsQueryOptions(),
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
	datacenters: Region[];
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
			content={{
				"initial-info": () => <StepInitialInfo />,
				deploy: () => <StepDeploy />,
			}}
			onSubmit={async ({ values }) => {
				const selectedDatacenters = Object.entries(values.datacenters)
					.filter(([, selected]) => selected)
					.map(([id]) => id);

				const config = {
					serverless: {
						url: values.endpoint,
						maxRunners: values.maxRunners,
						slotsPerRunner: values.slotsPerRunner,
						runnersMargin: values.runnerMargin,
						requestLifespan: VERCEL_SERVERLESS_MAX_DURATION - 5, // Subtract 5s to ensure we don't hit Vercel's timeout
						headers: Object.fromEntries(
							values.headers.map(([key, value]) => [key, value]),
						),
					},
					metadata: {
						provider: "vercel",
					},
				};

				const payload = Object.fromEntries(
					selectedDatacenters.map((dc) => [dc, config]),
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
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.id, true]),
				),
			}}
		/>
	);
}

function StepInitialInfo() {
	return (
		<>
			<div className="space-y-4">
				<p>Deploy the Rivet Vercel template to get started quickly.</p>
				<ExternalLinkCard
					href="https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-dev%2Ftemplate-vercel&env=NEXT_PUBLIC_RIVET_ENDPOINT,NEXT_PUBLIC_RIVET_TOKEN,NEXT_PUBLIC_RIVET_NAMESPACE&project-name=rivetkit-vercel&repository-name=rivetkit-vercel"
					icon={faVercel}
					title="Deploy Template to Vercel"
				/>
			</div>
			<div className="space-y-4">
				<p>Set the following environment variables:</p>
				<EnvVariablesStep />
			</div>
		</>
	);
}

function StepDeploy() {
	return (
		<>
			<div className="mt-2">
				<ConnectVercelForm.Endpoint />
				<Accordion type="single" collapsible>
					<AccordionItem value="item-1">
						<AccordionTrigger className="text-sm">
							Advanced
						</AccordionTrigger>
						<AccordionContent className="space-y-4 px-1 pt-2">
							<ConnectVercelForm.RunnerName />
							<ConnectVercelForm.Datacenters />
							<ConnectVercelForm.Headers />
							<ConnectVercelForm.SlotsPerRunner />
							<ConnectVercelForm.MinRunners />
							<ConnectVercelForm.MaxRunners />
							<ConnectVercelForm.RunnerMargin />
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</div>
			<ConnectVercelForm.ConnectionCheck provider="Vercel" />
		</>
	);
}
