import { faQuestionCircle, faRailway, Icon } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import * as EditRunnerConfigForm from "@/app/forms/edit-runner-config-form";
import { HelpDropdown } from "@/app/help-dropdown";
import { Button, type DialogContentProps, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

interface EditRunnerConfigFrameContentProps extends DialogContentProps {
	name: string;
}

export default function EditRunnerConfigFrameContent({
	name,
	onClose,
}: EditRunnerConfigFrameContentProps) {
	const { data } = useSuspenseQuery({
		...useEngineCompatDataProvider().runnerConfigQueryOptions(name),
		refetchInterval: 5000,
	});

	return (
		<EditRunnerConfigForm.Form
			onSubmit={async () => {
				onClose?.();
			}}
			defaultValues={{
				url: data.serverless.url,
				maxRunners: data.serverless.maxRunners,
				minRunners: data.serverless.minRunners,
				requestLifespan: data.serverless.requestLifespan,
				runnersMargin: data.serverless.runnersMargin,
				slotsPerRunner: data.serverless.slotsPerRunner,
			}}
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
				<EditRunnerConfigForm.Url />
				<div className="grid grid-cols-2">
					<EditRunnerConfigForm.MinRunners />
					<EditRunnerConfigForm.MaxRunners />
					<EditRunnerConfigForm.RunnersMargin />
				</div>
				<div className="grid grid-cols-2">
					<EditRunnerConfigForm.RequestLifespan />
					<EditRunnerConfigForm.RunnersMargin />
				</div>
				<EditRunnerConfigForm.SlotsPerRunner />
				<div className="flex justify-end mt-4">
					<EditRunnerConfigForm.Submit>
						Save
					</EditRunnerConfigForm.Submit>
				</div>
			</Frame.Content>
		</EditRunnerConfigForm.Form>
	);
}
