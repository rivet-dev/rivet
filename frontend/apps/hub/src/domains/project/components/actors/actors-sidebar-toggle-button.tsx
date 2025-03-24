import { Button } from "@rivet-gg/components";
import { Icon, faSidebar } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { useActorsLayout } from "./actors-layout-context";

export function ActorsSidebarToggleButton() {
	const { setFolded, isFolded } = useActorsLayout();
	return (
		<motion.div
			layout
			layoutId="actors-sidebar-toggle-button"
			animate={
				isFolded ? {} : { x: -30, opacity: 0, width: 0, margin: 0 }
			}
		>
			<Button
				variant="outline"
				size="icon-sm"
				onClick={() => setFolded(!isFolded)}
				className="mb-2 ml-1 mr-1"
			>
				<Icon icon={faSidebar} />
			</Button>
		</motion.div>
	);
}
