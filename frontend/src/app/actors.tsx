import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
	type ActorId,
	ActorNotFound,
	ActorsActorDetails,
	ActorsActorEmptyDetails,
	ActorsListPreview,
	useDataProvider,
} from "@/components/actors";

export function Actors({ actorId }: { actorId: string | undefined }) {
	return (
		<ActorsListPreview showDetails={!!actorId}>
			{actorId ? <Actor /> : <ActorsActorEmptyDetails />}
		</ActorsListPreview>
	);
}

function Actor() {
	const navigate = useNavigate();
	const { tab, actorId } = useSearch({ from: "/_context" });

	const { data, isError } = useQuery(
		useDataProvider().actorQueryOptions(actorId as ActorId),
	);

	if (!data || isError) {
		return <ActorNotFound actorId={actorId as ActorId} />;
	}

	return (
		<ActorsActorDetails
			actorId={actorId as ActorId}
			tab={tab}
			onTabChange={(tab) => {
				navigate({
					to: ".",
					search: (old) => ({ ...old, tab }),
				});
			}}
		/>
	);
}
