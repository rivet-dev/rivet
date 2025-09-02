import type { ReactNode } from "react";
import { cn, Skeleton } from "@/components";

interface PageLayoutProps {
	children: ReactNode;
	layout?: "compact" | "full" | "onboarding" | "actors" | "v2";
}

const PageLayout = ({ children, layout = "compact" }: PageLayoutProps) => (
	<div
		className={cn({
			"p-8 container": layout === "compact",
			"px-4 w-full h-full py-4":
				layout === "full" ||
				layout === "onboarding" ||
				layout === "actors",
			"w-full h-full": layout === "v2",
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
					"px-4 w-full h-full": layout === "v2",
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
