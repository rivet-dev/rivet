import type { ReactNode } from "react";
import { Page, type PageProps } from "./page";
import { SidebarPageContent } from "./sidebar-page-content";

export interface SidebarPageProps extends PageProps {
	sidebar?: ReactNode;
}

export const SidebarPage = ({
	children,
	sidebar,
	...props
}: SidebarPageProps) => {
	return (
		<Page {...props}>
			<SidebarPageContent sidebar={sidebar}>
				{children}
			</SidebarPageContent>
		</Page>
	);
};
