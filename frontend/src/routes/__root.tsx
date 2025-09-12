import type { Clerk } from "@clerk/clerk-js";
import * as Sentry from "@sentry/react";
import type { QueryClient } from "@tanstack/react-query";
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import posthog from "posthog-js";
import { match } from "ts-pattern";
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
		return match(__APP_TYPE__)
			.with("cloud", async () => {
				if (!context.clerk) return;

				if (context.clerk.status === "ready") {
					return;
				}

				// Wait for Clerk
				await new Promise((resolve, reject) => {
					context.clerk.on("status", (payload) => {
						if (payload === "ready") {
							Sentry.setUser({
								id: context.clerk.user?.id,
								email: context.clerk.user?.primaryEmailAddress
									?.emailAddress,
							});
							posthog.setPersonProperties({
								id: context.clerk.user?.id,
								email: context.clerk.user?.primaryEmailAddress
									?.emailAddress,
							});
							return resolve(true);
						}
					});
					// Timeout to avoid waiting indefinitely
					setTimeout(() => {
						reject(new Error("Can't confirm identity"));
					}, 10000);
				});
			})
			.otherwise(() => {
				// No-op for engine and inspector
			});
	},
});
