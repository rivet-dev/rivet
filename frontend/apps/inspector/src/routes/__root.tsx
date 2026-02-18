import type { QueryClient } from "@tanstack/react-query";
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import type { InspectorContext } from "@/app/data-providers/cache";
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

interface RootRouteContext {
	queryClient: QueryClient;
	getOrCreateInspectorContext: (opts: {
		url?: string;
		token?: string;
	}) => InspectorContext;
}

export const Route = createRootRouteWithContext<RootRouteContext>()({
	component: RootRoute,
	pendingComponent: FullscreenLoading,
});
