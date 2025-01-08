import {
	SidebarNavigation,
	SidebarPageContent,
	Skeleton,
} from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";

interface GroupPageProps {
	groupId: string;
	children: ReactNode;
}

function GroupSettingsPage({ children, groupId }: GroupPageProps) {
	return (
		<SidebarPageContent
			sidebar={
				<SidebarNavigation>
					<Link
						to="/teams/$groupId/settings"
						params={{ groupId }}
						className="text-foreground font-semibold"
					>
						General
					</Link>
				</SidebarNavigation>
			}
		>
			{children}
		</SidebarPageContent>
	);
}

GroupSettingsPage.Skeleton = function GroupSettingsPageSkeleton() {
	return (
		<SidebarPageContent
			sidebar={
				<SidebarNavigation>
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
					<Skeleton className="w-full h-5" />
				</SidebarNavigation>
			}
		>
			<Skeleton className="w-full h-56" />
			<Skeleton className="w-full h-56" />
		</SidebarPageContent>
	);
};

export { GroupSettingsPage as Root };
