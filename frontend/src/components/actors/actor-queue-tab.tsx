import { useQuery } from "@tanstack/react-query";
import { Info } from "./actor-state-tab";
import { useActorInspector } from "./actor-inspector-context";
import { useDataProvider } from "./data-provider";
import { ActorQueue } from "./actor-queue";
import type { ActorId } from "./queries";

interface ActorQueueTabProps {
	actorId: ActorId;
}

export function ActorQueueTab({ actorId }: ActorQueueTabProps) {
	const inspector = useActorInspector();
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	if (destroyedAt) {
		return (
			<Info>
				Queue data is unavailable for inactive Actors.
			</Info>
		);
	}

	if (!inspector.features.queue.supported) {
		return <Info>{inspector.features.queue.message}</Info>;
	}

	return <ActorQueue actorId={actorId} />;
}
