import {
	Alert,
	AlertDescription,
	AlertTitle,
	Button,
	Flex,
} from "@rivet-gg/components";
import { Icon, faGhost } from "@rivet-gg/icons";
import { Link } from "@tanstack/react-router";

interface GroupEmptyAlertProps {
	groupId: string;
	showCreateButton?: boolean;
}

export function GroupEmptyAlert({
	groupId,
	showCreateButton,
}: GroupEmptyAlertProps) {
	return (
		<>
			<Alert>
				<Icon className="size-4" icon={faGhost} />
				<AlertTitle>It's a ghost town!</AlertTitle>
				<AlertDescription>
					<Flex direction="col" items="start" gap="4">
						This group doesn't have any projects yet. Get started by
						creating a new one.
						{showCreateButton ? (
							<Button asChild variant="secondary">
								<Link
									to="/"
									search={{
										modal: "create-project",
										groupId,
									}}
								>
									Create a new project
								</Link>
							</Button>
						) : null}
					</Flex>
				</AlertDescription>
			</Alert>
		</>
	);
}
