import type { Clerk } from "@clerk/clerk-js";
import type { QueryClient } from "@tanstack/react-query";
import {
	createRootRouteWithContext,
	Outlet,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { FullscreenLoading } from "@/components";
import { waitForClerk } from "@/lib/waitForClerk";

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
	/**
	 * Only available in cloud mode
	 */
	clerk: Clerk;
	queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RootRouteContext>()({
	component: RootRoute,
	pendingComponent: FullscreenLoading,
	beforeLoad:async ({ context }) => {
		if (context.clerk && __APP_TYPE__ === "cloud") {
			return await waitForClerk(context.clerk);
		}
	}
});
