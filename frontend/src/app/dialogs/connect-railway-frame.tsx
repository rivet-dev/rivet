import { faQuestionCircle, faRailway, Icon } from "@rivet-gg/icons";
import * as ConnectRailwayForm from "@/app/forms/connect-railway-form";
import { HelpDropdown } from "@/app/help-dropdown";
import { Button, Flex, Frame } from "@/components";

export default function CreateProjectFrameContent() {
	return (
		<ConnectRailwayForm.Form
			onSubmit={async () => {}}
			defaultValues={{ name: "" }}
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
				<Flex gap="4" direction="col">
					<ConnectRailwayForm.Name />
					<ConnectRailwayForm.Preview />
					<ConnectRailwayForm.ConnectionCheck />
				</Flex>
			</Frame.Content>
		</ConnectRailwayForm.Form>
	);
}
