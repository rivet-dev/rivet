import { Page, Skeleton } from "@rivet-gg/components";
import type { PropsWithChildren, ReactNode } from "react";

interface GroupPageProps {
	children: ReactNode;
}

function GroupPage({ children }: GroupPageProps) {
	return <Page>{children}</Page>;
}

GroupPage.Skeleton = Page.Skeleton;

function Content({ children }: PropsWithChildren) {
	return children;
}
Content.Skeleton = () => {
	return (
		<>
			<Skeleton className="h-8 w-1/2" />
			<Skeleton className="h-64 w-full" />
		</>
	);
};

export { GroupPage as Root, Content };
