import { Button } from "@rivet-gg/components";
import { Icon, faSidebar } from "@rivet-gg/icons";
import { useActorsLayout } from "./actors-layout-context";

export function ActorsSidebarToggleButton() {
	const { setFolded, isFolded } = useActorsLayout();

	if (!isFolded) {
		return null;
	}
	return (
		<div>
			<Button
				variant="outline"
				size="icon-sm"
				onClick={() => setFolded(!isFolded)}
				className="mb-2 ml-1 mr-1"
			>
				<Icon icon={faSidebar} />
			</Button>
		</div>
	);
}
