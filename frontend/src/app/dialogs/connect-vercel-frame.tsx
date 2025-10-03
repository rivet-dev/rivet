import { faQuestionCircle, faVercel, Icon } from "@rivet-gg/icons";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { HelpDropdown } from "@/app/help-dropdown";
import {
	Button,
	type DialogContentProps,
	Frame,
} from "@/components";
import { defineStepper } from "@/components/ui/stepper";

const { Stepper } = defineStepper(
	{
		id: "step-1",
		title: "Select Vercel Plan",
	},
	{
		id: "step-2",
		title: "Edit vercel.json",
	},
	{
		id: "step-3",
		title: "Deploy to Vercel",
	},
	{
		id: "step-4",
		title: "Confirm Connection",
	},
);

interface CreateProjectFrameContentProps extends DialogContentProps {}

export default function CreateProjectFrameContent({
	onClose,
}: CreateProjectFrameContentProps) {
	return (
		<ConnectVercelForm.Form
			onSubmit={async () => {}}
			mode="onChange"
			revalidateMode="onChange"
			defaultValues={{ plan: "hobby", endpoint: "" }}
		>
			<Frame.Header>
				<Frame.Title className="justify-between flex items-center">
					<div>
						Add <Icon icon={faVercel} className="ml-0.5" />
						Vercel
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
		</ConnectVercelForm.Form>
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
								className="min-w-0"
								of={step.id}
								onClick={() => methods.goTo(step.id)}
							>
								<Stepper.Title>{step.title}</Stepper.Title>
								{methods.when(step.id, (step) => {
									return (
										<Stepper.Panel className="space-y-4">
											{step.id === "step-1" && (
												<ConnectVercelForm.Plan />
											)}
											{step.id === "step-2" && (
												<ConnectVercelForm.Json />
											)}
											{step.id === "step-3" && (
												<>
													<p>
														Deploy your project to
														Vercel using your
														favorite method. After
														deployment, return here
														to add the endpoint.
													</p>
												</>
											)}
											{step.id === "step-4" && (
												<div>
													<ConnectVercelForm.Endpoint className="mb-2" />
													<ConnectVercelForm.ConnectionCheck />
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
