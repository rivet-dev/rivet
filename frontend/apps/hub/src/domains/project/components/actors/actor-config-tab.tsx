import { Button, DocsSheet, ScrollArea } from "@rivet-gg/components";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ActorGeneral, type ActorGeneralProps } from "./actor-general";
import { ActorNetwork, type ActorNetworkProps } from "./actor-network";
import { ActorRuntime, type ActorRuntimeProps } from "./actor-runtime";

interface ActorConfigTabProps
	extends ActorRuntimeProps,
		ActorNetworkProps,
		ActorGeneralProps {}

export function ActorConfigTab(props: ActorConfigTabProps) {
	return (
		<ScrollArea className="overflow-auto h-full">
			<div className="flex justify-end items-center gap-1 border-b sticky top-0 p-2 bg-card z-[1]">
				<DocsSheet title="Config" path="docs/config">
					<Button
						variant="outline"
						size="sm"
						startIcon={<Icon icon={faBooks} />}
					>
						Documentation
					</Button>
				</DocsSheet>
			</div>
			<ActorGeneral {...props} />
			<ActorNetwork {...props} />
			<ActorRuntime {...props} />
		</ScrollArea>
	);
}
