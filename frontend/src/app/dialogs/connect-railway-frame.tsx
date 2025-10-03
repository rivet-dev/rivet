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
import { engineEnv } from "@/lib/env";

const { Stepper } = defineStepper(
	{
		id: "step-1",
		title: "Deploy to Railway",
	},
	{
		id: "step-2",
		title: "Set Environment Variables",
	},
	{
		id: "step-3",
		title: "Confirm Connection",
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
				<Frame.Title className="justify-between flex items-center">
					<div>
						Add <Icon icon={faRailway} className="ml-0.5" /> Railway
					</div>
					<HelpDropdown>
						<Button variant="ghost" size="icon">
							<Icon icon={faQuestionCircle} />
						</Button>
					</HelpDropdown>
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
														href="https://railway.com/deploy/rivet?referralCode=RC7bza&utm_medium=integration&utm_source=template&utm_campaign=generic"
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

													<p>
														After deploying your app
														to Railway, return here
														to add the endpoint.
													</p>
												</>
											)}
											{step.id === "step-2" && (
												<EnvVariablesStep />
											)}
											{step.id === "step-3" && (
												<div>
													<ConnectRailwayForm.Endpoint
														placeholder="https://your-application.up.railway.app/"
														className="mb-2"
													/>
													<ConnectRailwayForm.ConnectionCheck />
												</div>
											)}
											<Stepper.Controls>
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
		dataProvider.connectRunnerTokenQueryOptions(),
	);

	return (
		<>
			<p>
				Set the following environment variables in your Railway project
				settings.
			</p>
			<div className="gap-1 items-center">
				{__APP_TYPE__ === "engine" ? (
					<div className="flex gap-1 items-center border-b border-b-border/40 pb-2 mb-2">
						<DiscreteInput value="RIVET_ENGINE" show />
						<DiscreteInput value={engineEnv().VITE_APP_API_URL} />
					</div>
				) : null}
				<div className="flex gap-1 items-center">
					<DiscreteInput value="RIVET_TOKEN" show />
					{isLoading ? (
						<Skeleton className="w-56 h-10" />
					) : (
						<DiscreteInput value={data || ""} />
					)}
				</div>
			</div>
		</>
	);
}
