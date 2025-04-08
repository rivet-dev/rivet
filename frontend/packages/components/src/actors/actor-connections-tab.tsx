import { LiveBadge, ScrollArea } from "@rivet-gg/components";
import { ActorObjectInspector } from "./console/actor-inspector";
import {
	useActorConnections,
	useActorWorkerStatus,
} from "./worker/actor-worker-context";
import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";

const selector = (a: Actor) => a.destroyedAt;

interface ActorConnectionsTabProps {
	actor: ActorAtom;
}

export function ActorConnectionsTab({ actor }: ActorConnectionsTabProps) {
	const destroyedAt = useAtomValue(selectAtom(actor, selector));
	const status = useActorWorkerStatus();

	const connections = useActorConnections();

	if (destroyedAt) {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				Connections Preview is unavailable for inactive Actors.
			</div>
		);
	}

	if (status.type === "error") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				Connections Preview is currently unavailable.
				<br />
				See console/logs for more details.
			</div>
		);
	}

	if (status.type !== "ready") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				Loading connections...
			</div>
		);
	}

	return (
		<ScrollArea className="flex-1 w-full min-h-0 h-full">
			<div className="flex  justify-between items-center gap-1 border-b sticky top-0 p-2 bg-card z-[1]">
				<LiveBadge />
			</div>
			<div className="p-2">
				<ActorObjectInspector
					name="connections"
					data={Object.fromEntries(connections.map((c) => [c.id, c]))}
					expandPaths={["$"]}
				/>
			</div>
		</ScrollArea>
	);
}
