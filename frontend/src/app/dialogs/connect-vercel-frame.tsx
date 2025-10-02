import { faQuestionCircle, faVercel, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { HelpDropdown } from "@/app/help-dropdown";
import { Button, Flex, Frame } from "@/components";
import { useEngineCompatDataProvider } from "@/components/actors";

export default function CreateProjectFrameContent() {
	const provider = useEngineCompatDataProvider();

	const { mutateAsync } = useMutation(
		provider.createRunnerConfigMutationOptions(),
	);

	return (
		<ConnectVercelForm.Form
			onSubmit={async (values) => {
				await mutateAsync({
					name: values.name,
					config: {
						serverless: {
							url: values.endpoint,
						},
					},
				});
			}}
			defaultValues={{ name: "" }}
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
				<Flex gap="4" direction="col">
					<ConnectVercelForm.Plan />
					<ConnectVercelForm.Endpoint />
				</Flex>
			</Frame.Content>
			<Frame.Footer>
				<ConnectVercelForm.Submit type="submit">
					Add
				</ConnectVercelForm.Submit>
			</Frame.Footer>
		</ConnectVercelForm.Form>
	);
}
