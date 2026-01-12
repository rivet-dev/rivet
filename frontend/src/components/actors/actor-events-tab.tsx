import { useQuery } from "@tanstack/react-query";
import { ActorEvents } from "./actor-events";
import { useActorInspector } from "./actor-inspector-context";
import { Info } from "./actor-state-tab";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorEventsTabProps {
	actorId: ActorId;
}

export function ActorEventsTab({ actorId }: ActorEventsTabProps) {
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	const { isError, isLoading } = useQuery(
		useActorInspector().actorEventsQueryOptions(actorId),
	);

	if (destroyedAt) {
		return (
			<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center col-span-full py-8">
				State Preview is unavailable for inactive Actors.
			</div>
		);
	}

	if (isError) {
		return (
			<Info>
				Database Studio is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	if (isLoading) {
		return <Info>Loading...</Info>;
	}

	return <ActorEvents actorId={actorId} />;
}
