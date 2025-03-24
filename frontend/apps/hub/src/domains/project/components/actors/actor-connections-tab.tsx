import { Button, DocsSheet, LiveBadge, ScrollArea } from "@rivet-gg/components";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ActorObjectInspector } from "./console/actor-inspector";
import {
	useActorConnections,
	useActorWorkerStatus,
} from "./worker/actor-worker-context";
interface ActorConnectionsTabProps {
	disabled?: boolean;
}

export function ActorConnectionsTab({ disabled }: ActorConnectionsTabProps) {
	const status = useActorWorkerStatus();

	const connections = useActorConnections();

	if (disabled) {
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

	if (status.type === "unsupported") {
		return (
			<div className="flex-1 flex items-center justify-center h-full text-xs text-center">
				Connections Preview is not supported for this Actor.
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
				<DocsSheet title="Connections" path="docs/connections">
					<Button
						variant="outline"
						size="sm"
						startIcon={<Icon icon={faBooks} />}
					>
						Documentation
					</Button>
				</DocsSheet>
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
