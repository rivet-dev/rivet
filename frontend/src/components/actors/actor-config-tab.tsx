import { faBooks, Icon } from "@rivet-gg/icons";
import { Button, ScrollArea } from "@/components";
import { ActorGeneral } from "./actor-general";
import { ActorRunner } from "./actor-runner";
import type { ActorId } from "./queries";

interface ActorConfigTabProps {
	actorId: ActorId;
}

export function ActorConfigTab(props: ActorConfigTabProps) {
	return (
		<ScrollArea className="overflow-auto h-full">
			<div className="flex justify-end items-center gap-1 border-b sticky top-0 p-2 z-[1] h-[45px]">
				<Button
					variant="outline"
					size="sm"
					startIcon={<Icon icon={faBooks} />}
					asChild
				>
					<a
						href="https://rivet.dev/docs/config"
						target="_blank"
						rel="noopener noreferrer"
					>
						Documentation
					</a>
				</Button>
			</div>
			<ActorGeneral {...props} />
		</ScrollArea>
	);
}
