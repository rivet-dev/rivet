import { useQuery } from "@tanstack/react-query";
import { useActorInspector } from "./actor-inspector-context";
import { Info } from "./actor-state-tab";
import { ActorTraces } from "./actor-traces";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorTracesTabProps {
	actorId: ActorId;
}

export function ActorTracesTab({ actorId }: ActorTracesTabProps) {
	const inspector = useActorInspector();
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	if (destroyedAt) {
		return <Info>Traces are unavailable for inactive Actors.</Info>;
	}

	if (!inspector.features.traces.supported) {
		return <Info>{inspector.features.traces.message}</Info>;
	}

	return <ActorTraces actorId={actorId} />;
}
