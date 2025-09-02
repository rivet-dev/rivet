import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import {
	ActorFeature,
	type ActorId,
	ActorNotFound,
	ActorsActorDetails,
	ActorsActorEmptyDetails,
	ActorsListPreview,
	useManager,
} from "@/components/actors";

export function Actors({ actorId }: { actorId: string | undefined }) {
	return (
		<ActorsListPreview showDetails={!!actorId}>
			{actorId ? (
				<Actor />
			) : (
				<ActorsActorEmptyDetails
					features={[
						ActorFeature.Config,
						ActorFeature.State,
						ActorFeature.Connections,
					]}
				/>
			)}
		</ActorsListPreview>
	);
}

function Actor() {
	const navigate = useNavigate();
	const { tab, actorId } = useSearch({ from: "/_layout" });

	const { data, isError } = useQuery(
		useManager().actorQueryOptions(actorId as ActorId),
	);

	if (!data || isError) {
		return (
			<ActorNotFound
				actorId={actorId as ActorId}
				features={[
					ActorFeature.Config,
					ActorFeature.State,
					ActorFeature.Connections,
				]}
			/>
		);
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
