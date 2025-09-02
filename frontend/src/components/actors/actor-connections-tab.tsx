import { useQuery } from "@tanstack/react-query";
import { LiveBadge, ScrollArea } from "@/components";
import { useActor } from "./actor-queries-context";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useManager } from "./manager-context";
import { type ActorId, useActorConnectionsStream } from "./queries";

interface ActorConnectionsTabProps {
	actorId: ActorId;
}

export function ActorConnectionsTab({ actorId }: ActorConnectionsTabProps) {
	const { data: destroyedAt } = useQuery(
		useManager().actorDestroyedAtQueryOptions(actorId),
	);

	const actorQueries = useActor();
	const {
		data: { connections } = {},
		isError,
		isLoading,
	} = useQuery(actorQueries.actorConnectionsQueryOptions(actorId));

	// useActorConnectionsStream(actorId);

	if (destroyedAt) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-center">
				Connections Preview is unavailable for inactive Actors.
			</div>
		);
	}

	if (isError) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-center">
				Connections Preview is currently unavailable.
				<br />
				See console/logs for more details.
			</div>
		);
	}

	if (isLoading) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-center">
				Loading connections...
			</div>
		);
	}

	return (
		<ScrollArea className="flex-1 w-full min-h-0 h-full">
			<div className="flex  justify-between items-center gap-1 border-b sticky top-0 p-2 z-[1] h-[45px]">
				<LiveBadge />
			</div>
			<div className="p-2">
				<ActorObjectInspector
					name="connections"
					data={connections}
					expandPaths={["$"]}
				/>
			</div>
		</ScrollArea>
	);
}
