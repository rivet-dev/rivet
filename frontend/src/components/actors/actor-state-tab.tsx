import { useQuery } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { DocsSheet } from "../docs-sheet";
import { Button } from "../ui/button";
import { ActorEditableState } from "./actor-editable-state";
import { useActor } from "./actor-queries-context";
import { useActorsView } from "./actors-view-context-provider";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

interface ActorStateTabProps {
	actorId: ActorId;
}

export function ActorStateTab({ actorId }: ActorStateTabProps) {
	const { data: destroyedAt } = useQuery(
		useManager().actorDestroyedAtQueryOptions(actorId),
	);

	const { links } = useActorsView();

	const actorQueries = useActor();
	const {
		data: state,
		isError,
		isLoading,
	} = useQuery(
		actorQueries.actorStateQueryOptions(actorId, { enabled: !destroyedAt }),
	);

	if (destroyedAt) {
		return <Info>State Preview is unavailable for inactive Actors.</Info>;
	}

	if (isError) {
		return (
			<Info>
				State Preview is currently unavailable.
				<br />
				See console/logs for more details.
			</Info>
		);
	}

	if (isLoading) {
		return <Info>Loading state...</Info>;
	}

	if (!state?.enabled) {
		return (
			<Info>
				<p>
					State Preview is not enabled for this Actor. <br /> You can
					enable it by providing a valid state constructor.
				</p>
				<DocsSheet title="State" path={links.state}>
					<Button variant="outline">Documentation</Button>
				</DocsSheet>
			</Info>
		);
	}

	return (
		<div className="flex-1 w-full min-h-0 h-full flex flex-col">
			<ActorEditableState actorId={actorId} state={state.state} />
		</div>
	);
}

export function Info({ children }: PropsWithChildren) {
	return (
		<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center">
			{children}
		</div>
	);
}
