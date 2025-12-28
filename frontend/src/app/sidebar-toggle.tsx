import { faSidebar, Icon } from "@rivet-gg/icons";
import { Button, WithTooltip } from "@/components";
import { useRootLayout } from "@/components/actors/root-layout-context";

export function SidebarToggle({ className }: { className?: string }) {
	const { isSidebarCollapsed, sidebarRef } = useRootLayout();

	if (isSidebarCollapsed) {
		return (
			<WithTooltip
				delayDuration={0}
				trigger={
					<Button
						onClick={() => sidebarRef.current?.expand()}
						variant="outline"
						size="icon-sm"
						className={className}
					>
						<Icon icon={faSidebar} />
					</Button>
				}
				content="Show sidebar"
			/>
		);
	}
	return null;
}
