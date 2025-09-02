import { useQuery } from "@tanstack/react-query";
import { useManager } from "./manager-context";
import type { ActorId, ActorStatus } from "./queries";

export const ACTOR_STATUS_LABEL_MAP = {
	unknown: "Unknown",
	starting: "Starting",
	running: "Running",
	stopped: "Stopped",
	crashed: "Crashed",
	sleeping: "Sleeping",
} satisfies Record<ActorStatus, string>;

export const ActorStatusLabel = ({ status }: { status?: ActorStatus }) => {
	return <span>{status ? ACTOR_STATUS_LABEL_MAP[status] : "Unknown"}</span>;
};

export const QueriedActorStatusLabel = ({ actorId }: { actorId: ActorId }) => {
	const { data: status, isError } = useQuery(
		useManager().actorStatusQueryOptions(actorId),
	);
	return <ActorStatusLabel status={isError ? "unknown" : status} />;
};
