import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { FullscreenLoading } from "@/components";

function RootRoute() {
	return (
		<>
			<Outlet />
			{import.meta.env.DEV ? (
				<TanStackRouterDevtools position="bottom-right" />
			) : null}
		</>
	);
}

export const Route = createRootRouteWithContext()({
	component: RootRoute,
	pendingComponent: FullscreenLoading,
	wrapInSuspense: true,
});
