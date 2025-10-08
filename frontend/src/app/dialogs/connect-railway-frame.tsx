import { faQuestionCircle, faRailway, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import { HelpDropdown } from "@/app/help-dropdown";
import {
	Button,
	type DialogContentProps,
	DiscreteInput,
	Frame,
	Skeleton,
} from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";
import { defineStepper } from "@/components/ui/stepper";
import { cloudEnv, engineEnv } from "@/lib/env";
import { NeedHelp } from "../forms/connect-vercel-form";

const { Stepper } = defineStepper(
	{
		id: "step-1",
		title: "Deploy to Railway",
	},
	{
		id: "step-3",
		title: "Wait for a Runner to connect",
	},
);

interface ConnectRailwayFrameContentProps extends DialogContentProps {}

export default function ConnectRailwayFrameContent({
	onClose,
}: ConnectRailwayFrameContentProps) {
	return (
		<ConnectRailwayForm.Form
			onSubmit={async () => {}}
			defaultValues={{ endpoint: "" }}
		>
			<Frame.Header>
				<Frame.Title className="gap-2 flex items-center">
					<div>
						Add <Icon icon={faRailway} className="ml-0.5" /> Railway
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
	const dataProvider = useEngineCompatDataProvider();

	const { data } = useQuery(dataProvider.engineAdminTokenQueryOptions());

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
											{step.id === "step-1" && (
												<>
													<p>
														Deploy any RivetKit app
														to Railway.
													</p>
													<p>
														Or use our Railway
														template to get started
														quickly.
													</p>
													<a
														href={`https://railway.com/new/template/rivet-cloud-starter?referralCode=RC7bza&utm_medium=integration&utm_source=template&utm_campaign=generic&RIVET_TOKEN=${data}&RIVET_ENDPOINT=${
															engineEnv()
																.VITE_APP_API_URL
														}&RIVET_NAMESPACE=${
															dataProvider.engineNamespace
														}`}
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

													<EnvVariablesStep />
												</>
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
	const dataProvider = useEngineCompatDataProvider();

	const { data, isLoading } = useQuery(
		dataProvider.engineAdminTokenQueryOptions(),
	);

	return (
		<>
			<p>
				Set the following environment variables in your Railway project
				settings.
			</p>
			<div className="gap-1 items-center grid grid-cols-2">
				<DiscreteInput value="RIVET_ENDPOINT" show />
				<DiscreteInput value={engineEnv().VITE_APP_API_URL} />
				<DiscreteInput value="RIVET_TOKEN" show />
				{isLoading ? (
					<Skeleton className="w-56 h-10" />
				) : (
					<DiscreteInput value={data || ""} />
				)}
				<DiscreteInput value="RIVET_NAMESPACE" show />
				<DiscreteInput value={dataProvider.engineNamespace} show />
			</div>
		</>
	);
}
