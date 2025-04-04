import {
	ActorsActorDetails,
	ActorsActorDetailsPanel,
	ActorsListPreview,
	currentActorAtom,
} from "@rivet-gg/components/actors";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useAtomValue } from "jotai";

export function Actors({ actorId }: { actorId: string | undefined }) {
	return (
		<ActorsListPreview>
			<ActorsActorDetailsPanel actorId={actorId}>
				{actorId ? <Actor /> : null}
			</ActorsActorDetailsPanel>
		</ActorsListPreview>
	);
}

function Actor() {
	const actor = useAtomValue(currentActorAtom);
	const navigate = useNavigate();
	const { tab } = useSearch({ from: "/_layout/" });

	if (!actor) {
		return null;
	}
	return (
		<ActorsActorDetails
			actor={actor}
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
