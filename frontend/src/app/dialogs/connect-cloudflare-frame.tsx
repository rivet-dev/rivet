import { faCloudflare, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import * as ConnectCloudflareForm from "@/app/forms/connect-cloudflare-form";
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
import { StepperForm } from "../forms/stepper-form";

// Cloudflare Workers has a 30-second CPU time limit on the free plan
// and up to 15 minutes on paid plans with Durable Objects
export const CLOUDFLARE_WORKERS_MAX_DURATION = 30;

const { stepper } = ConnectCloudflareForm;

interface ConnectCloudflareFrameContentProps extends DialogContentProps {}

export default function ConnectCloudflareFrameContent({
	onClose,
}: ConnectCloudflareFrameContentProps) {
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
						Add <Icon icon={faCloudflare} className="ml-0.5" />
						Cloudflare Workers
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
				install: () => <StepInstall />,
				configure: () => <StepConfigure />,
				handler: () => <StepHandler />,
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
						requestLifespan: CLOUDFLARE_WORKERS_MAX_DURATION - 5, // Subtract 5s to ensure we don't hit Cloudflare's timeout
						headers: Object.fromEntries(
							values.headers.map(([key, value]) => [key, value]),
						),
					},
					metadata: {
						provider: "cloudflare",
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

function StepInstall() {
	return <ConnectCloudflareForm.InstallCode />;
}

function StepConfigure() {
	return <ConnectCloudflareForm.WranglerConfig />;
}

function StepHandler() {
	return <ConnectCloudflareForm.HandlerCode />;
}

function StepDeploy() {
	return (
		<>
			<p>
				Deploy your code to Cloudflare and paste your deployment's endpoint:
			</p>
			<div className="mt-2">
				<ConnectCloudflareForm.Endpoint
					placeholder="https://my-rivet-app.workers.dev/api/rivet"
				/>
				<Accordion type="single" collapsible>
					<AccordionItem value="item-1">
						<AccordionTrigger className="text-sm">
							Advanced
						</AccordionTrigger>
						<AccordionContent className="space-y-4 px-1 pt-2">
							<ConnectCloudflareForm.RunnerName />
							<ConnectCloudflareForm.Datacenters />
							<ConnectCloudflareForm.Headers />
							<ConnectCloudflareForm.SlotsPerRunner />
							<ConnectCloudflareForm.MinRunners />
							<ConnectCloudflareForm.MaxRunners />
							<ConnectCloudflareForm.RunnerMargin />
						</AccordionContent>
					</AccordionItem>
				</Accordion>
			</div>
			<ConnectCloudflareForm.ConnectionCheck provider="Cloudflare" />
			<p className="text-muted-foreground text-sm">
				Need help deploying? See{" "}
				<a
					href="https://www.rivet.dev/docs/actors/quickstart/cloudflare-workers/"
					target="_blank"
					rel="noreferrer"
					className="underline"
				>
					Cloudflare Workers deployment documentation
				</a>
				.
			</p>
		</>
	);
}
