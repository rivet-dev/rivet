import { Skeleton } from "@rivet-gg/components";
import type { ReactNode } from "react";

interface PageCenteredProps {
	children: ReactNode;
}

const PageCentered = ({ children }: PageCenteredProps) => (
	<div className="flex flex-1 items-center justify-center">
		<div className="max-w-sm w-full">{children}</div>
	</div>
);

PageCentered.Skeleton = function PageCenteredSkeleton() {
	return (
		<PageCentered>
			<Skeleton className="w-full h-96 max-w-[21.75rem]" />
		</PageCentered>
	);
};

export { PageCentered as Root };
