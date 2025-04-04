import { Button, DocsSheet, ScrollArea } from "@rivet-gg/components";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ActorGeneral } from "./actor-general";
import { ActorNetwork } from "./actor-network";
import { ActorRuntime } from "./actor-runtime";
import type { ActorAtom } from "./actor-context";

interface ActorConfigTabProps {
	actor: ActorAtom;
}

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
