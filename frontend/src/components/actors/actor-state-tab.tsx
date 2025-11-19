import { useQuery } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";
import { DocsSheet } from "../docs-sheet";
import { Button } from "../ui/button";
import { ActorEditableState } from "./actor-editable-state";
import { useActor } from "./actor-queries-context";
import { useActorsView } from "./actors-view-context-provider";
import { useActorInspector } from "./inspector-context";
import type { ActorId } from "./queries";

interface ActorStateTabProps {
	actorId: ActorId;
}

export function ActorStateTab({ actorId }: ActorStateTabProps) {
	const { links } = useActorsView();

	const actorQueries = useActorInspector();

	const {
		data: isStateEnabled,
		isError,
		isLoading,
	} = useQuery(actorQueries.actorIsStateEnabledQueryOptions(actorId));

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

	if (!isStateEnabled) {
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
			<ActorEditableState actorId={actorId} />
		</div>
	);
}

export function Info({ children }: PropsWithChildren) {
	return (
		<div className="flex-1 flex flex-col gap-2 items-center justify-center h-full text-center max-w-lg mx-auto">
			{children}
		</div>
	);
}
