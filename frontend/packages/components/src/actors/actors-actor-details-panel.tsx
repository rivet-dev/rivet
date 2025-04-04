import { CatchBoundary } from "@tanstack/react-router";
import { ActorsActorDetails } from "./actors-actor-details";
import { ActorsActorMissing } from "./actors-actor-missing";
import { ActorsActorError } from "./actors-actor-not-found";
import type { ReactNode } from "react";

interface ActorsActorDetailsPanelProps {
	actorId: string | undefined;
	children: ReactNode;
}

export function ActorsActorDetailsPanel({
	actorId,
	children,
}: ActorsActorDetailsPanelProps) {
	if (!actorId) {
		return <ActorsActorMissing />;
	}

	return (
		// <CatchBoundary
		// 	getResetKey={() => actorId}
		// 	errorComponent={ActorsActorError}
		// >
		children
		// </CatchBoundary>
	);
}
// @ts-ignore
ActorsActorDetailsPanel.Skeleton = ActorsActorDetails.Skeleton;
