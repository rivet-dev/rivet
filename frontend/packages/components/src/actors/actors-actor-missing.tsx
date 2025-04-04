import { Button } from "@rivet-gg/components";
import { useActorsLayout } from "./actors-layout-context";
import { ActorsSidebarToggleButton } from "./actors-sidebar-toggle-button";

export function ActorsActorMissing() {
	const { setFolded, isFolded } = useActorsLayout();
	return (
		<div className="flex-1 h-full min-h-0 overflow-auto flex flex-col">
			<div className="flex justify-between items-center border-b pt-2">
				<ActorsSidebarToggleButton />
			</div>
			<div className="flex-1 flex items-center justify-center text-xs text-center flex-col gap-1">
				Please select an Actor from the list.
				{isFolded ? (
					<Button
						variant="outline"
						size="sm"
						onClick={() => setFolded(false)}
					>
						Show Actor List
					</Button>
				) : null}
			</div>
		</div>
	);
}
