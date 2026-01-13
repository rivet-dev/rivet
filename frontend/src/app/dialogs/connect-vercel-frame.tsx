import { faVercel, Icon } from "@rivet-gg/icons";
import {
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import confetti from "canvas-confetti";
import { useWatch } from "react-hook-form";
import { match } from "ts-pattern";
import type z from "zod";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
	type DialogContentProps,
	Frame,
	getConfig,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { cloudEnv } from "@/lib/env";
import { usePublishableToken } from "@/queries/accessors";
import { queryClient } from "@/queries/global";
import { type JoinStepSchemas, StepperForm } from "../forms/stepper-form";

const { stepper } = ConnectVercelForm;

type FormValues = z.infer<JoinStepSchemas<typeof stepper.steps>>;

export const VERCEL_SERVERLESS_MAX_DURATION = 300;

interface CreateProjectFrameContentProps extends DialogContentProps {}

const useEndpoint = () => {
	return match(__APP_TYPE__)
		.with("cloud", () => {
			return cloudEnv().VITE_APP_API_URL;
		})
		.with("engine", () => {
			return getConfig().apiUrl;
		})
		.otherwise(() => {
			throw new Error("Not in a valid context");
		});
};

export default function CreateProjectFrameContent({
	onClose,
}: CreateProjectFrameContentProps) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().datacentersQueryOptions(),
		pages: Infinity,
	});

	const { data: datacenters } = useSuspenseInfiniteQuery(
		useEngineCompatDataProvider().datacentersQueryOptions(),
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
	const token = usePublishableToken();
	const endpoint = useEndpoint();
	const namespace = provider.engineNamespace;

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
				// "initial-info": () => <StepInitialInfo />,
				"api-route": () => <StepApiRoute />,
				frontend: () => (
					<StepFrontend
						token={token}
						endpoint={endpoint}
						namespace={namespace}
					/>
				),
				variables: () => (
					<>
						<p>
							Set these variables in Settings &gt; Environment
							Variables in the Vercel dashboard.
						</p>
						<ConnectVercelForm.EnvVariables />
					</>
				),
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
				plan: "hobby",
				runnerName: "default",
				slotsPerRunner: 1,
				minRunners: 1,
				maxRunners: 10_000,
				runnerMargin: 0,
				headers: [],
				success: false,
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.name, true]),
				),
			}}
		/>
	);
}

// function StepInitialInfo() {
// 	return (
// 		<>
// 			<ConnectVercelForm.Plan />
// 			<Accordion type="single" collapsible>
// 				<AccordionItem value="item-1">
// 					<AccordionTrigger className="text-sm">
// 						Advanced
// 					</AccordionTrigger>
// 					<AccordionContent className="space-y-4 px-1 pt-2">
// 						<ConnectVercelForm.RunnerName />
// 						<ConnectVercelForm.Datacenters />
// 						<ConnectVercelForm.Headers />
// 						<ConnectVercelForm.SlotsPerRunner />
// 						<ConnectVercelForm.MinRunners />
// 						<ConnectVercelForm.MaxRunners />
// 						<ConnectVercelForm.RunnerMargin />
// 					</AccordionContent>
// 				</AccordionItem>
// 			</Accordion>
// 		</>
// 	);
// }

function StepApiRoute() {
	const plan = useWatch({ name: "plan" });
	return <ConnectVercelForm.IntegrationCode plan={plan || "hobby"} />;
}

function StepFrontend({
	token,
	endpoint,
	namespace,
}: {
	token: string;
	endpoint: string;
	namespace: string;
}) {
	return (
		<ConnectVercelForm.FrontendIntegrationCode
			token={token}
			endpoint={endpoint}
			namespace={namespace}
		/>
	);
}

function StepDeploy() {
	return (
		<>
			<p>
				Deploy your code to Vercel and paste your deployment's endpoint:
			</p>
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
			<p className="text-muted-foreground text-sm">
				Need help deploying? See{" "}
				<a
					href="https://vercel.com/docs/deployments"
					target="_blank"
					rel="noreferrer"
					className="underline"
				>
					Vercel's deployment documentation
				</a>
				.
			</p>
		</>
	);
}
