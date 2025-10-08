import { faQuestionCircle, faVercel, Icon } from "@rivet-gg/icons";
import { useFormContext } from "react-hook-form";
import z from "zod";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { HelpDropdown } from "@/app/help-dropdown";
import { Button, type DialogContentProps, Frame } from "@/components";
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
				<Frame.Title className="gap-2 flex items-center">
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
														<a
															href="https://vercel.com/docs/deployments"
															target="_blank"
															rel="noreferrer"
															className=" underline"
														>
															Deploy your project
															to Vercel using your
															preferred method
														</a>
														. After deployment,
														return here to add the
														endpoint.
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
												{step.id === "step-4" ? (
													<NeedHelpButton />
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
													type="button"
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

function NeedHelpButton() {
	const { watch } = useFormContext();
	const endpoint = watch("endpoint");
	const enabled = !!endpoint && z.string().url().safeParse(endpoint).success;

	if (enabled) {
		return <ConnectVercelForm.NeedHelp />;
	}

	return null;
}
