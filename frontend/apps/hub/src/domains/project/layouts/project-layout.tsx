import { computePageLayout } from "@/lib/compute-page-layout";
import { Page, Skeleton } from "@rivet-gg/components";
import { useMatches } from "@tanstack/react-router";
import type { PropsWithChildren, ReactNode } from "react";

interface ProjectPageProps {
	children: ReactNode;
}

function ProjectPage({ children }: ProjectPageProps) {
	const matches = useMatches();
	return <Page layout={computePageLayout(matches)}>{children}</Page>;
}

ProjectPage.Skeleton = Page.Skeleton;

function Content({ children }: PropsWithChildren) {
	return <>{children}</>;
}

Content.Skeleton = function ContentSkeleton() {
	return (
		<>
			<Skeleton className="my-4 h-12 w-1/3" />
			<div className="flex flex-row gap-4">
				<Skeleton className="h-64 w-2/3" />
				<Skeleton className="h-64 w-1/3" />
			</div>
			<Skeleton className="h-64 w-full" />
			<Skeleton className="h-64 w-full" />
		</>
	);
};

function EmptyProjectPage({ children }: PropsWithChildren) {
	const matches = useMatches();
	return <Page layout={computePageLayout(matches)}>{children}</Page>;
}

export { ProjectPage as Root, EmptyProjectPage as EmptyRoot, Content };
