import { faRailway, faServer, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useFormContext } from "react-hook-form";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import {
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
import { NeedHelp } from "../forms/connect-vercel-form";

const { Stepper } = defineStepper(
	{
		id: "initial",
		title: "Get Started",
	},
	{
		id: "step-1",
		title: "Deploy",
	},
	{
		id: "step-3",
		title: "Wait for the Runner to connect",
	},
);

interface ConnectManualFrameContentProps extends DialogContentProps {}

export default function ConnectManualFrameContent({
	onClose,
}: ConnectManualFrameContentProps) {
	return (
		<ConnectRailwayForm.Form
			onSubmit={async () => {}}
			mode="onChange"
			defaultValues={{
				runnerName: "default",
				datacenter: "auto",
			}}
		>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faServer} className="ml-0.5" /> Manual
						Runner
					</div>
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<FormStepper onClose={onClose} />
			</Frame.Content>
		</ConnectRailwayForm.Form>
	);
}

function FormStepper({ onClose }: { onClose?: () => void }) {
	return (
		<Stepper.Provider variant="vertical">
			{({ methods }) => (
				<>
					<Stepper.Navigation>
						{methods.all.map((step) => (
							<Stepper.Step
								key={step.id}
								className="min-w-0"
								of={step.id}
								onClick={() => methods.goTo(step.id)}
							>
								<Stepper.Title>{step.title}</Stepper.Title>
								{methods.when(step.id, (step) => {
									return (
										<Stepper.Panel className="space-y-4">
											{step.id === "initial" && (
												<>
													<p>
														Connect any
														RivetKit-compatible
														runner.
													</p>
													<ConnectRailwayForm.RunnerName />
													<ConnectRailwayForm.Datacenter />
												</>
											)}
											{step.id === "step-1" && (
												<EnvVariablesStep />
											)}
											{step.id === "step-3" && (
												<div>
													<ConnectRailwayForm.ConnectionCheck />
												</div>
											)}
											<Stepper.Controls>
												{step.id === "step-3" ? (
													<NeedHelp />
												) : null}
												<Button
													type="button"
													variant="secondary"
													onClick={methods.prev}
													disabled={methods.isFirst}
												>
													Previous
												</Button>
												<Button
													onClick={
														methods.isLast
															? onClose
															: methods.next
													}
												>
													{methods.isLast
														? "Done"
														: "Next"}
												</Button>
											</Stepper.Controls>
										</Stepper.Panel>
									);
								})}
							</Stepper.Step>
						))}
					</Stepper.Navigation>
				</>
			)}
		</Stepper.Provider>
	);
}

function EnvVariablesStep() {
	return (
		<>
			<p>Set the following environment variables in your deployment.</p>
			<div>
				<div
					className="gap-1 items-center grid grid-cols-2"
					data-env-variables
				>
					<Label className="text-muted-foreground text-xs mb-1">
						Key
					</Label>
					<Label className="text-muted-foreground text-xs mb-1">
						Value
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
						<Button variant="ghost" size="sm">
							Copy all raw
						</Button>
					</CopyButton>
				</div>
			</div>
		</>
	);
}

function RivetRunnerEnv() {
	const { watch } = useFormContext();

	const runnerName = watch("runnerName");
	if (runnerName === "rivetkit") return null;

	return (
		<>
			<DiscreteInput
				aria-label="environment variable key"
				value="RIVET_RUNNER"
				show
			/>
			<DiscreteInput
				aria-label="environment variable value"
				value={runnerName}
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
					value={data || ""}
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
	const { watch } = useFormContext();
	const datacenter = watch("datacenter");

	const { data } = useQuery(
		useEngineCompatDataProvider().regionQueryOptions(datacenter),
	);

	return data?.url || engineEnv().VITE_APP_API_URL;
};
