import { useQuery } from "@tanstack/react-query";
import { ActorDatabase } from "./actor-database";
import { useActor } from "./actor-queries-context";
import { Info } from "./actor-state-tab";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

interface ActorDatabaseTabProps {
	actorId: ActorId;
}

export function ActorDatabaseTab({ actorId }: ActorDatabaseTabProps) {
	const { data: destroyedAt } = useQuery(
		useManager().actorDestroyedAtQueryOptions(actorId),
	);

	const actorQueries = useActor();
	const {
		data: isEnabled,
		isLoading,
		isError,
	} = useQuery(actorQueries.actorDatabaseEnabledQueryOptions(actorId));

	if (destroyedAt) {
		return <Info>Database Studio is unavailable for inactive Actors.</Info>;
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

	if (!isEnabled) {
		return (
			<Info>
				<p>
					Database Studio is not enabled for this Actor. <br /> You
					can enable it by providing a valid database connection
					provider.
				</p>
			</Info>
		);
	}

	return (
		<div className="flex-1 w-full min-h-0 min-w-0 h-full flex flex-col">
			<ActorDatabase actorId={actorId} />
		</div>
	);
}
