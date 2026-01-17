import { faCloudflare, Icon } from "@rivet-gg/icons";
import { useMutation, usePrefetchInfiniteQuery } from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import z from "zod";
import * as ConnectServerlessForm from "@/app/forms/connect-manual-serverless-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	CodeFrame,
	CodeGroup,
	CodePreview,
	type DialogContentProps,
	Frame,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { successfulBackendSetupEffect } from "@/lib/effects";
import { queryClient } from "@/queries/global";
import { EnvVariables } from "../env-variables";
import { StepperForm } from "../forms/stepper-form";
import {
	endpointSchema,
	ServerlessConnectionCheck,
} from "../serverless-connection-check";
import { useEndpoint } from "./connect-manual-serverfull-frame";

const CLOUDFLARE_MAX_REQUEST_DURATION = 30;

const stepper = defineStepper(
	{
		id: "configure",
		title: "Configure runner",
		assist: false,
		next: "Next",
		schema: z.object({
			runnerName: z.string().min(1, "Runner name is required"),
			datacenters: z
				.record(z.boolean())
				.refine(
					(data) => Object.values(data).some(Boolean),
					"At least one datacenter must be selected",
				),
			headers: z.array(z.tuple([z.string(), z.string()])).default([]),
			slotsPerRunner: z.coerce.number().min(1, "Must be at least 1"),
			maxRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
			requestLifespan: z.coerce
				.number()
				.min(1, "Must be at least 1")
				.max(
					CLOUDFLARE_MAX_REQUEST_DURATION,
					"Cloudflare Workers requests time out after 30s",
				),
		}),
	},
	{
		id: "deploy",
		title: "Deploy to Cloudflare Workers",
		assist: false,
		next: "Next",
		schema: z.object({}),
	},
	{
		id: "verify",
		title: "Connect & verify",
		assist: true,
		next: "Add",
		schema: z.object({
			endpoint: endpointSchema,
			success: z.boolean().refine((v) => v === true, {
				message: "Runner must be connected to proceed",
			}),
		}),
	},
);

interface ConnectCloudflareFrameContentProps extends DialogContentProps {
	title?: React.ReactNode;
	footer?: React.ReactNode;
}

export default function ConnectCloudflareFrameContent({
	onClose,
	title,
	footer,
}: ConnectCloudflareFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	return (
		<>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					{title ?? (
						<div>
							Add <Icon icon={faCloudflare} className="ml-0.5" />{" "}
							Cloudflare Workers
						</div>
					)}
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<FormStepper onClose={onClose} footer={footer} />
			</Frame.Content>
		</>
	);
}

function FormStepper({
	onClose,
	footer,
}: {
	onClose?: () => void;
	footer?: React.ReactNode;
}) {
	const provider = useEngineCompatDataProvider();

	const { mutateAsync } = useMutation({
		...provider.upsertRunnerConfigMutationOptions(),
		onSuccess: async () => {
			successfulBackendSetupEffect();
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
				configure: () => <StepConfigure />,
				deploy: () => <StepDeploy />,
				verify: () => <StepVerify />,
			}}
			onSubmit={async ({ values }) => {
				const selectedDatacenters = Object.entries(values.datacenters)
					.filter(([, selected]) => selected)
					.map(([id]) => id);

				const config = {
					serverless: {
						url: values.endpoint,
						maxRunners: values.maxRunners,
						minRunners: values.minRunners,
						slotsPerRunner: values.slotsPerRunner,
						runnersMargin: values.runnerMargin,
						requestLifespan: values.requestLifespan,
						headers: Object.fromEntries(
							values.headers.map(([key, value]) => [key, value]),
						),
					},
					metadata: { provider: "cloudflare-workers" },
				};

				const payload = Object.fromEntries(
					selectedDatacenters.map((dc) => [dc, config]),
				);

				await mutateAsync({ name: values.runnerName, config: payload });
			}}
			defaultValues={{
				runnerName: "default",
				slotsPerRunner: 1,
				minRunners: 1,
				maxRunners: 10_000,
				runnerMargin: 0,
				requestLifespan: 25,
				headers: [],
				success: false,
				endpoint: "",
				datacenters: {},
			}}
			footer={footer}
		/>
	);
}

function StepConfigure() {
	return (
		<div className="space-y-4">
			<ConnectServerlessForm.RunnerName />
			<ConnectServerlessForm.Datacenters />
			<Accordion type="single" collapsible>
				<AccordionItem value="advanced">
					<AccordionTrigger className="text-sm">
						Advanced
					</AccordionTrigger>
					<AccordionContent className="space-y-4 px-1 pt-2">
						<ConnectServerlessForm.Headers />
						<ConnectServerlessForm.SlotsPerRunner />
						<ConnectServerlessForm.MinRunners />
						<ConnectServerlessForm.MaxRunners />
						<ConnectServerlessForm.RunnerMargin />
						<ConnectServerlessForm.RequestLifespan />
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	);
}

function StepDeploy() {
	const runnerName = useWatch({ name: "runnerName" });
	return (
		<div className="space-y-4">
			<p>
				Make sure your Worker is integrated with RivetKit. See the{" "}
				<a
					href="https://www.rivet.dev/docs/connect/cloudflare-workers/"
					target="_blank"
					rel="noopener noreferrer"
					className="underline"
				>
					Cloudflare Workers guide
				</a>{" "}
				for wiring up Durable Objects.
			</p>
			<p>Set these environment variables in your Wrangler config:</p>
			<EnvVariables endpoint={useEndpoint()} runnerName={runnerName} />
			<div className="space-y-2">
				<p>Deploy to Cloudflare's edge:</p>
				<CodeGroup>
					{[
						<CodeFrame
							key="wrangler-deploy"
							title="wrangler"
							language="bash"
							code={() => "wrangler deploy"}
						>
							<CodePreview
								className="w-full min-w-0"
								language="bash"
								code="wrangler deploy"
							/>
						</CodeFrame>,
					]}
				</CodeGroup>
			</div>
			<p className="text-sm text-muted-foreground">
				Use your deployed Worker URL with{" "}
				<span className="font-mono">/rivet</span> appended for the
				endpoint.
			</p>
		</div>
	);
}

function StepVerify() {
	return (
		<>
			<p>
				Paste the deployed Worker endpoint (including /rivet) and wait
				for the health check to pass.
			</p>
			<ConnectServerlessForm.Endpoint placeholder="https://my-worker.workers.dev/rivet" />
			<ServerlessConnectionCheck provider="cloudflare-workers" />
		</>
	);
}
