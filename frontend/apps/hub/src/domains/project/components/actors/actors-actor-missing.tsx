import { ActorsSidebarToggleButton } from "./actors-sidebar-toggle-button";

export function ActorsActorMissing() {
	return (
		<div className="flex-1 h-full min-h-0 overflow-auto flex flex-col">
			<div className="flex justify-between items-center border-b min-h-9 mt-2">
				<ActorsSidebarToggleButton />
			</div>
			<div className="flex flex-1 items-center justify-center text-xs text-center flex-col gap-1">
				Please select an Actor from the list.
			</div>
		</div>
	);
}
