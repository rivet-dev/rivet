import { faQuestionSquare, Icon } from "@rivet-gg/icons";
import { ActorDetailsSkeleton } from "./actor-details-skeleton";
import { useActorsView } from "./actors-view-context-provider";

// The top-level `ActorsActorDetails` is exported from `actor-details-iframe.tsx`
// — that's the dashboard wrapper that mounts the actor's bundled inspector UI
// inside an iframe. This file holds the no-actor-selected placeholder, which
// renders a disabled tab strip + a hint to pick an actor from the list.

export const ActorsActorEmptyDetails = () => {
	const { copy } = useActorsView();
	return (
		<div className="flex flex-col h-full w-full min-w-0 min-h-0 flex-1">
			<ActorDetailsSkeleton>
				<div className="flex text-center text-foreground flex-1 justify-center items-center flex-col gap-2">
					<Icon icon={faQuestionSquare} className="text-4xl" />
					<p className="max-w-[400px]">{copy.selectActor}</p>
				</div>
			</ActorDetailsSkeleton>
		</div>
	);
};
