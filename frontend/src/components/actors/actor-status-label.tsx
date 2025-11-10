import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { RelativeTime } from "../relative-time";
import { useDataProvider } from "./data-provider";
import type { ActorId, ActorStatus } from "./queries";

export const ACTOR_STATUS_LABEL_MAP = {
	unknown: "Unknown",
	starting: "Starting",
	running: "Running",
	stopped: "Stopped",
	crashed: "Crashed",
	sleeping: "Sleeping",
	pending: "Pending",
	"crash-loop": "Crash Loop Backoff",
} satisfies Record<ActorStatus, string>;

export const ActorStatusLabel = ({ status }: { status?: ActorStatus }) => {
	return (
		<span className="whitespace-nowrap">
			{status ? ACTOR_STATUS_LABEL_MAP[status] : "Unknown"}
		</span>
	);
};

export const QueriedActorStatusLabel = ({
	actorId,
	showAdditionalInfo,
}: {
	actorId: ActorId;
	showAdditionalInfo?: boolean;
}) => {
	const { data: status, isError } = useQuery(
		useDataProvider().actorStatusQueryOptions(actorId),
	);
	return (
		<>
			<ActorStatusLabel status={isError ? "unknown" : status} />
			{showAdditionalInfo && (
				<QueriedActorStatusAdditionalInfo actorId={actorId} />
			)}
		</>
	);
};

export function QueriedActorStatusAdditionalInfo({
	actorId,
}: {
	actorId: ActorId;
}) {
	const { data: { rescheduleAt } = {} } = useQuery(
		useDataProvider().actorStatusAdditionalInfoQueryOptions(actorId),
	);

	if (rescheduleAt) {
		return (
			<span>
				Will try to start again{" "}
				<span>
					<RelativeTime time={new Date(rescheduleAt)} /> (
					{formatISO(rescheduleAt)}){" "}
				</span>
			</span>
		);
	}

	return null;
}
