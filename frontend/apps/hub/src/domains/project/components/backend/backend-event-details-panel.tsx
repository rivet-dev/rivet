import { Flex, Text } from "@rivet-gg/components";
import { BackendEventDetails } from "./backend-event-details";

interface BackendEventDetailsPanelProps {
	environmentId: string;
	projectId: string;
	eventId: string | undefined;
}

export function BackendEventDetailsPanel({
	eventId,
	projectId,
	environmentId,
}: BackendEventDetailsPanelProps) {
	if (!eventId) {
		return (
			<Flex items="center" justify="center" className="h-full">
				<Text textAlign="center">
					Please select an event from the list on the left.
				</Text>
			</Flex>
		);
	}
	return (
		<BackendEventDetails
			environmentId={environmentId}
			projectId={projectId}
			eventId={eventId}
		/>
	);
}
