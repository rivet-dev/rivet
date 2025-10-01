import { ClerkProvider } from "@clerk/clerk-react";
import { dark } from "@clerk/themes";
import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { match } from "ts-pattern";
import { FullscreenLoading } from "@/components";
import { clerk } from "@/lib/auth";

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

function CloudRootRoute() {
	const PUBLISHABLE_KEY = import.meta.env.VITE_CLERK_PUBLISHABLE_KEY;

	if (!PUBLISHABLE_KEY) {
		throw new Error("Add your Clerk Publishable Key to the .env file");
	}

	return (
		<ClerkProvider
			Clerk={clerk}
			publishableKey={PUBLISHABLE_KEY}
			appearance={{ theme: dark }}
		>
			<RootRoute />
		</ClerkProvider>
	);
}

export const Route = createRootRouteWithContext()({
	component: match(__APP_TYPE__)
		.with("cloud", () => CloudRootRoute)
		.otherwise(() => RootRoute),
	pendingComponent: FullscreenLoading,
	wrapInSuspense: true,
});
