import { faVercel, Icon } from "@rivet-gg/icons";
import { useMutation } from "@tanstack/react-query";
import * as ConnectVercelForm from "@/app/forms/connect-vercel-form";
import { Flex, Frame } from "@/components";
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
				<Frame.Title>
					Add <Icon icon={faVercel} className="ml-0.5" />
					Vercel
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<ConnectVercelForm.Name />
					<ConnectVercelForm.Endpoint />
					<ConnectVercelForm.Preview />
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
