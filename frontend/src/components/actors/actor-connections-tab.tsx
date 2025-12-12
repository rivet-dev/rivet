import { useQuery } from "@tanstack/react-query";
import { LiveBadge, ScrollArea } from "@/components";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useDataProvider } from "./data-provider";
import { useActorInspector } from "./inspector-context";
import type { ActorId } from "./queries";

interface ActorConnectionsTabProps {
	actorId: ActorId;
}

export function ActorConnectionsTab({ actorId }: ActorConnectionsTabProps) {
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	const inspector = useActorInspector();
	const { data = [] } = useQuery(
		inspector.actorConnectionsQueryOptions(actorId),
	);

	if (destroyedAt) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-center">
				Connections Preview is unavailable for inactive Actors.
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
					data={data}
					expandPaths={["$"]}
				/>
			</div>
		</ScrollArea>
	);
}
