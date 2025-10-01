import { faRailway, Icon } from "@rivet-gg/icons";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import { Flex, Frame } from "@/components";

export default function CreateProjectFrameContent() {
	return (
		<ConnectRailwayForm.Form
			onSubmit={async () => {}}
			defaultValues={{ name: "" }}
		>
			<Frame.Header>
				<Frame.Title>
					Add <Icon icon={faRailway} className="ml-0.5" /> Railway
				</Frame.Title>
			</Frame.Header>
			<Frame.Content>
				<Flex gap="4" direction="col">
					<ConnectRailwayForm.Name />
					<ConnectRailwayForm.Preview />
					<ConnectRailwayForm.ConnectionCheck />
				</Flex>
			</Frame.Content>
		</ConnectRailwayForm.Form>
	);
}
