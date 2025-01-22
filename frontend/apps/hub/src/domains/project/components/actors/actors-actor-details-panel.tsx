import { CatchBoundary, useNavigate, useSearch } from "@tanstack/react-router";
import { ActorsActorDetails } from "./actors-actor-details";
import { ActorsActorMissing } from "./actors-actor-missing";
import { ActorsActorError } from "./actors-actor-not-found";

interface ActorsActorDetailsPanelProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string | undefined;
}

export function ActorsActorDetailsPanel({
	projectNameId,
	environmentNameId,
	actorId,
}: ActorsActorDetailsPanelProps) {
	const currentTab = useSearch({
		from: "/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors",
		select: (state) => state.tab,
	});

	const navigate = useNavigate();

	if (!actorId) {
		return <ActorsActorMissing />;
	}

	return (
		<CatchBoundary
			getResetKey={() => actorId}
			errorComponent={ActorsActorError}
		>
			<ActorsActorDetails
				tab={currentTab}
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				actorId={actorId}
				onTabChange={(tab) => {
					navigate({
						from: "/projects/$projectNameId/environments/$environmentNameId/actors",
						to: ".",
						search: (old) => ({ ...old, tab }),
					});
				}}
			/>
		</CatchBoundary>
	);
}
ActorsActorDetailsPanel.Skeleton = ActorsActorDetails.Skeleton;
