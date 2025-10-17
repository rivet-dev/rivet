import { faVercel, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import type z from "zod";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	type DialogContentProps,
	Frame,
} from "@/components";
import { type Region, useEngineCompatDataProvider } from "@/components/actors";
import { queryClient } from "@/queries/global";
import { type JoinStepSchemas, StepperForm } from "../forms/stepper-form";

const { stepper } = ConnectVercelForm;

type FormValues = z.infer<JoinStepSchemas<typeof stepper.steps>>;

interface CreateProjectFrameContentProps extends DialogContentProps {}

export default function CreateProjectFrameContent({
	onClose,
}: CreateProjectFrameContentProps) {
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
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 />,
				"step-3": () => <Step3 />,
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
						requestLifespan:
							ConnectVercelForm.PLAN_TO_MAX_DURATION[values.plan],
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
				plan: "hobby",
				runnerName: "default",
				slotsPerRunner: 25,
				minRunners: 1,
				maxRunners: 1000,
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

function Step1() {
	return (
		<>
			<ConnectVercelForm.Plan />
			<Accordion type="single" collapsible>
				<AccordionItem value="item-1">
					<AccordionTrigger className="text-sm">
						Advanced options
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
		</>
	);
}

function Step2() {
	const plan = useWatch<FormValues>({ name: "plan" as const });
	return <ConnectVercelForm.Json plan={plan || "hobby"} />;
}

function Step3() {
	return (
		<>
			<p>
				<a
					href="https://vercel.com/docs/deployments"
					target="_blank"
					rel="noreferrer"
					className=" underline"
				>
					Deploy your project to Vercel using your preferred method
				</a>
				. After deployment, return here to add the endpoint.
			</p>
			<div className="mt-2">
				<ConnectVercelForm.Endpoint />
				<ConnectVercelForm.ConnectionCheck provider="Vercel" />
			</div>
		</>
	);
}
