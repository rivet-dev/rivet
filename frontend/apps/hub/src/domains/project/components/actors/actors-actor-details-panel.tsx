import { Flex, Text } from "@rivet-gg/components";
import { ActorsActorDetails } from "./actors-actor-details";

interface ActorsActorDetailsPanelProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string | undefined;
}

export function ActorsActorDetailsPanel({
	projectNameId,
	environmentNameId,
	actorId,
}: ActorsActorDetailsPanelProps) {
	if (!actorId) {
		return (
			<Flex items="center" justify="center" className="h-full">
				<Text textAlign="center">
					Please select an actor from the list.
				</Text>
			</Flex>
		);
	}

	return (
		<ActorsActorDetails
			projectNameId={projectNameId}
			environmentNameId={environmentNameId}
			actorId={actorId}
		/>
	);
}
ActorsActorDetailsPanel.Skeleton = ActorsActorDetails.Skeleton;
