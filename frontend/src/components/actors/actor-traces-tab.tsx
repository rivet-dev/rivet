import { useQuery } from "@tanstack/react-query";
import { Info } from "./actor-state-tab";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";
import { ActorTraces } from "./actor-traces";
import { useActorInspector } from "./actor-inspector-context";

interface ActorTracesTabProps {
	actorId: ActorId;
}

export function ActorTracesTab({ actorId }: ActorTracesTabProps) {
	const inspector = useActorInspector();
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	if (destroyedAt) {
		return (
			<Info>
				Traces are unavailable for inactive Actors.
			</Info>
		);
	}

	if (!inspector.features.traces.supported) {
		return <Info>{inspector.features.traces.message}</Info>;
	}

	return <ActorTraces actorId={actorId} />;
}
