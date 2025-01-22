import { Skeleton, cn } from "@rivet-gg/components";
import type { ReactNode } from "react";

interface PageLayoutProps {
	children: ReactNode;
	layout?: "compact" | "full" | "onboarding" | "actors";
}

const PageLayout = ({ children, layout = "compact" }: PageLayoutProps) => (
	<div
		className={cn({
			"p-8 container": layout === "compact",
			"px-4 w-full h-full py-4":
				layout === "full" ||
				layout === "onboarding" ||
				layout === "actors",
		})}
	>
		{children}
	</div>
);

const PageLayoutSkeleton = ({
	layout = "compact",
}: Pick<PageLayoutProps, "layout">) => {
	return (
		<div
			className={cn(
				{
					container: layout === "compact",
					"px-8 w-full h-full":
						layout === "full" || layout === "actors",
				},
				"pt-4",
			)}
		>
			<Skeleton className="my-8 h-12 w-1/3" />
			<div className="mb-4 flex flex-row gap-4">
				<Skeleton className="h-64 w-2/3" />
				<Skeleton className="h-64 w-1/3" />
			</div>
			<Skeleton className="mb-4 h-64 w-full" />
			<Skeleton className="h-64 w-full" />
		</div>
	);
};

PageLayout.Skeleton = PageLayoutSkeleton;

export { PageLayout as Root };
