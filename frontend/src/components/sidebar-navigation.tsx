import type { ReactNode } from "react";
import { cn } from "./lib/utils";
import { type CommonHelperProps, getCommonHelperClass } from "./ui/helpers";

export interface SidebarNavigationProps extends Partial<CommonHelperProps> {
	children: ReactNode;
}

export function SidebarNavigation({
	children,
	...props
}: SidebarNavigationProps) {
	return (
		<nav
			className={cn(
				"flex flex-col gap-4 text-sm text-muted-foreground",
				getCommonHelperClass(props),
			)}
		>
			{children}
		</nav>
	);
}
