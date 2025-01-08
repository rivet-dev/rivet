import { SidebarNavigation, SidebarPageContent } from "@rivet-gg/components";
import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";

interface GroupPageProps {
	projectNameId: string;
	children: ReactNode;
}

function GroupSettingsPage({ children, projectNameId }: GroupPageProps) {
	return (
		<SidebarPageContent
			sidebar={
				<SidebarNavigation>
					<Link
						to="/projects/$projectNameId/settings"
						activeOptions={{ exact: true }}
						params={{ projectNameId }}
						className="aria-current-page:text-foreground aria-current-page:font-semibold"
					>
						Tokens
					</Link>
				</SidebarNavigation>
			}
		>
			{children}
		</SidebarPageContent>
	);
}

export { GroupSettingsPage as Root };
