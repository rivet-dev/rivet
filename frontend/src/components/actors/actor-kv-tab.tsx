import { useQuery } from "@tanstack/react-query";
import { ActorKv } from "./actor-kv";
import { useActor } from "./actor-queries-context";
import { Info } from "./actor-state-tab";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorKvTabProps {
	actorId: ActorId;
}

export function ActorKvTab({ actorId }: ActorKvTabProps) {
	const { data: destroyedAt } = useQuery(
		useDataProvider().actorDestroyedAtQueryOptions(actorId),
	);

	const { isError, isLoading } = useQuery(
		useActor().actorKvQueryOptions(actorId),
	);

	if (destroyedAt) {
		return (
			<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center col-span-full py-8">
				KV Inspector is unavailable for inactive Actors.
			</div>
		);
	}

	if (isError) {
		return (
			<Info>
				KV Inspector is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	if (isLoading) {
		return <Info>Loading...</Info>;
	}

	return <ActorKv actorId={actorId} />;
}
