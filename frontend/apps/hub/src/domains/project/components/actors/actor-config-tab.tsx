import { ScrollArea } from "@rivet-gg/components";
import { ActorNetwork, type ActorNetworkProps } from "./actor-network";
import { ActorRuntime, type ActorRuntimeProps } from "./actor-runtime";

interface ActorConfigTabProps extends ActorRuntimeProps, ActorNetworkProps {}

export function ActorConfigTab(props: ActorConfigTabProps) {
	return (
		<ScrollArea className="overflow-auto h-full px-4 my-2">
			<ActorNetwork {...props} />
			<ActorRuntime {...props} />
		</ScrollArea>
	);
}
