import {
	faCreditCard,
	faGear,
	faHouse,
	faLogs,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/components/lib/utils";

export type ClusterView = "overview" | "logs" | "billing" | "settings";

export const CLUSTER_VIEWS: {
	id: ClusterView;
	label: string;
	icon: IconProp;
}[] = [
	{ id: "overview", label: "Overview", icon: faHouse },
	{ id: "logs", label: "Logs", icon: faLogs },
	{ id: "billing", label: "Billing", icon: faCreditCard },
	{ id: "settings", label: "Settings", icon: faGear },
];

interface ClusterSidebarProps {
	view: ClusterView;
	onViewChange: (view: ClusterView) => void;
}

/**
 * Left sidebar, same chrome as the real dashboard `Sidebar` (ghost nav
 * buttons).
 */
export function ClusterSidebar({ view, onViewChange }: ClusterSidebarProps) {
	return (
		<aside className="flex w-56 shrink-0 flex-col gap-2 bg-background px-2 pt-1.5">
			<nav className="flex flex-col gap-0.5">
				{CLUSTER_VIEWS.map((item) => (
					<SidebarButton
						key={item.id}
						icon={item.icon}
						active={view === item.id}
						onClick={() => onViewChange(item.id)}
					>
						{item.label}
					</SidebarButton>
				))}
			</nav>
		</aside>
	);
}

/** Mirrors `HeaderLink` in `@/app/layout`: ghost button, active = foreground tint. */
function SidebarButton({
	icon,
	active,
	className,
	children,
	onClick,
}: {
	icon: IconProp;
	active: boolean;
	className?: string;
	children: ReactNode;
	onClick: () => void;
}) {
	return (
		<Button
			variant="ghost"
			// `data-active:` styles key off the attribute TanStack `Link` sets.
			data-status={active ? "active" : undefined}
			aria-current={active ? "page" : undefined}
			onClick={onClick}
			className={cn(
				"h-auto justify-start px-1 py-1 font-medium text-muted-foreground hover:bg-foreground/[0.04] data-active:bg-foreground/[0.06] data-active:text-foreground",
				className,
			)}
			startIcon={
				<Icon
					className="size-5 opacity-80 transition-opacity group-hover:opacity-100"
					icon={icon}
				/>
			}
		>
			{children}
		</Button>
	);
}
