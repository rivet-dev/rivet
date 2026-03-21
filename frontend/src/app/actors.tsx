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
	const { tab, actorId, n, actorKey } = useSearch({ from: "/_context" });

	const id =
		actorKey && n ? { key: actorKey ?? "", name: n?.[0] ?? "" } : actorId;

	const { data, isError } = useQuery(useDataProvider().actorQueryOptions(id));

	if (!data || isError) {
		return (
			<ActorNotFound
				actorId={actorId as ActorId}
				name={n?.[0]}
				actorKey={actorKey}
			/>
		);
	}

	return (
		<ActorsActorDetails
			actorId={data.actorId}
			tab={tab}
			onTabChange={(tab) =>
				navigate({
					to: ".",
					search: (old) => ({ ...old, tab }),
				})
			}
		/>
	);
}
