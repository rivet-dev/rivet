import { faCheck, faGoogleCloud, faSpinnerThird, Icon } from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useMutation,
	usePrefetchInfiniteQuery,
	useSuspenseInfiniteQuery,
} from "@tanstack/react-query";
import confetti from "canvas-confetti";
import { useEffect } from "react";
import { useController, useFormContext } from "react-hook-form";
import z from "zod";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { cn, type DialogContentProps, Frame } from "@/components";
import { type Region, useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { queryClient } from "@/queries/global";
import { StepperForm } from "../forms/stepper-form";
import { EnvVariablesStep } from "./connect-railway-frame";

const stepper = defineStepper(
	{
		id: "step-1",
		title: "Configure",
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
			maxRunners: z.coerce.number().min(1, "Must be at least 1"),
			minRunners: z.coerce.number().min(0, "Must be 0 or greater"),
			runnerMargin: z.coerce.number().min(0, "Must be 0 or greater"),
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
		title: "Confirm Connection",
		assist: false,
		schema: z.object({
			success: z.boolean().refine((v) => v === true, {
				message: "Runner must be connected to proceed",
			}),
		}),
		next: "Add",
	},
);

interface ConnectAwsFrameContentProps extends DialogContentProps {}

export default function ConnectAwsFrameContent({
	onClose,
}: ConnectAwsFrameContentProps) {
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
						Add <Icon icon={faGoogleCloud} className="ml-0.5" />{" "}
						Google Cloud Run
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
	onClose,
	datacenters,
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
			onSubmit={async ({ values }) => {
				const selectedDatacenters = Object.entries(values.datacenters)
					.filter(([, selected]) => selected)
					.map(([id]) => id);

				const config = Object.fromEntries(
					selectedDatacenters.map((dc) => [
						dc,
						{
							normal: {},
							metadata: { provider: "gcp" },
						},
					]),
				);

				await mutateAsync({
					name: values.runnerName,
					config,
				});
			}}
			defaultValues={{
				runnerName: "default",
				slotsPerRunner: 25,
				maxRunners: 1000,
				runnerMargin: 0,
				headers: [],
				success: false,
				datacenters: Object.fromEntries(
					datacenters.map((dc) => [dc.id, true]),
				),
			}}
			content={{
				"step-1": () => <Step1 />,
				"step-2": () => <Step2 />,
				"step-3": () => <Step3 />,
			}}
		/>
	);
}

function Step1() {
	return (
		<div className="space-y-4">
			<ConnectVercelForm.RunnerName />
			<ConnectVercelForm.Datacenters />
			<ConnectVercelForm.Headers />
			<ConnectVercelForm.SlotsPerRunner />
			<ConnectVercelForm.MaxRunners />
			<ConnectVercelForm.MinRunners />
			<ConnectVercelForm.RunnerMargin />
		</div>
	);
}

function Step2() {
	return (
		<>
			<p>Set the following environment variables.</p>
			<EnvVariablesStep />
		</>
	);
}

function Step3({ provider = "gcp" }: { provider?: string }) {
	usePrefetchInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		pages: Infinity,
	});

	const { data: queryData } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		refetchInterval: 1000,
		maxPages: Infinity,
	});

	const { watch } = useFormContext();

	const datacenters: Record<string, boolean> = watch("datacenters");
	const chosenDatacenters = Object.entries(datacenters)
		.filter(([, enabled]) => enabled)
		.map(([dc]) => dc);

	const runnerName: string = watch("runnerName");

	const success = chosenDatacenters
		.map((dc) =>
			queryData?.find(
				(runner) =>
					runner.datacenter === dc && runner.name === runnerName,
			),
		)
		.every((v) => v);

	const {
		field: { onChange },
	} = useController({ name: "success" });

	useEffect(() => {
		onChange(success);
	}, [success]);

	return (
		<div
			className={cn(
				"text-center h-24 text-muted-foreground text-sm overflow-hidden flex items-center justify-center",
				success && "text-primary-foreground",
			)}
		>
			{success ? (
				<>
					<Icon icon={faCheck} className="mr-1.5 text-primary" />{" "}
					Runner successfully connected
				</>
			) : (
				<div className="flex flex-col items-center gap-2">
					<div className="flex items-center">
						<Icon
							icon={faSpinnerThird}
							className="mr-1.5 animate-spin"
						/>{" "}
						Waiting for Runner to connect...
					</div>
				</div>
			)}
		</div>
	);
}
