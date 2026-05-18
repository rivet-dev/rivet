import { ScrollArea } from "@/components";
import { ActorGeneral } from "./actor-general";
import type { ActorId } from "./queries";

interface ActorConfigTabProps {
	actorId: ActorId;
}

export function ActorConfigTab(props: ActorConfigTabProps) {
	return (
		<ScrollArea className="overflow-auto h-full">
			<ActorGeneral {...props} />
		</ScrollArea>
	);
}
