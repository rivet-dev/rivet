// import { isRivetError } from "@/lib/utils";
// import { RivetError } from "@rivet-gg/api";
import { faCircleExclamation, Icon } from "@rivet-gg/icons";
import type { ErrorComponentProps } from "@tanstack/react-router";
import { ActorsSidebarToggleButton } from "./actors-sidebar-toggle-button";

export function ActorsActorError({ error }: ErrorComponentProps) {
	// if (isRivetError(error) || error instanceof RivetError) {
	// 	return (
	// 		<div className="flex-1 h-full min-h-0 overflow-auto flex flex-col">
	// 			<div className="flex justify-between items-center border-b min-h-9 mt-2">
	// 				<ActorsSidebarToggleButton />
	// 			</div>
	// 			<div className="flex flex-1 items-center justify-center text-xs text-center flex-col gap-1">
	// 				<Icon icon={faNotdef} className="text-xl" />
	// 				Actor not found.
	// 			</div>
	// 		</div>
	// 	);
	// }

	return (
		<div className="flex-1 h-full min-h-0 overflow-auto flex flex-col">
			<div className="flex justify-between items-center border-b min-h-9 mt-2">
				<ActorsSidebarToggleButton />
			</div>
			<div className="flex flex-1 items-center justify-center text-xs text-center flex-col gap-1">
				<Icon icon={faCircleExclamation} className="text-xl" />
				Error occurred while fetching Actor.
			</div>
		</div>
	);
}
