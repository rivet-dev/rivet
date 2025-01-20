import {
	Button,
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
	Text,
} from "@rivet-gg/components";
import { Icon, faWifiSlash } from "@rivet-gg/icons";

export const NetworkIssueError = () => {
	return (
		<Card w="full">
			<CardHeader>
				<CardTitle className="flex gap-2">
					<Icon icon={faWifiSlash} />
					Connection issue!
				</CardTitle>
			</CardHeader>
			<CardContent>
				<Text>
					It seems that you do not have working network connection, or
					one of your extension blocks hub from accessing required
					resources.
					<br />
					Check your network connection, disable browser extensions
					and try again. If the issue still persist, please contact
					us.
				</Text>
			</CardContent>
			<CardFooter>
				<Button
					onClick={() => {
						window.location.reload();
					}}
				>
					Refresh
				</Button>
			</CardFooter>
		</Card>
	);
};
