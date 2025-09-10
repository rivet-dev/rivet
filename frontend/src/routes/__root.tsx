import type { Clerk } from "@clerk/clerk-js";
import type { QueryClient } from "@tanstack/react-query";
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import posthog from "posthog-js";
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
	/**
	 * Only available in cloud mode
	 */
	clerk: Clerk;
	queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RootRouteContext>()({
	component: RootRoute,
	pendingComponent: FullscreenLoading,
	beforeLoad: async ({ context }) => {
		if (!context.clerk) return;

		// wait for Clerk
		await new Promise((resolve, reject) => {
			context.clerk.on("status", (payload) => {
				if (payload === "ready") {
					posthog.setPersonProperties({
						id: context.clerk.user?.id,
						email: context.clerk.user?.primaryEmailAddress
							?.emailAddress,
					});
					return resolve(true);
				}
				// If the status is not "ready", we don't resolve the promise
				// We can also add a timeout to avoid waiting indefinitely
				setTimeout(() => {
					reject(new Error("Can't confirm identity"));
				}, 10000);
			});
		});
	},
});
