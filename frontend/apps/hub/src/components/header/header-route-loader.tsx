import { useRouterState } from "@tanstack/react-router";
import { ShimmerLine } from "../shimmer-line";

export function HeaderRouteLoader() {
	const isLoading = useRouterState({
		select: (s) => s.isLoading || s.isTransitioning,
	});

	if (!isLoading) {
		return null;
	}
	return <ShimmerLine className="-bottom-1" />;
}
