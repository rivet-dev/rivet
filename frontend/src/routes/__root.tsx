import type { Clerk } from "@clerk/clerk-js";
import { ClerkProvider } from "@clerk/clerk-react";
import * as ClerkComponents from "@clerk/elements/common";
import { dark } from "@clerk/themes";
import type { QueryClient } from "@tanstack/react-query";
import {
	createRootRouteWithContext,
	Outlet,
	useNavigate,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { match } from "ts-pattern";
import { FullscreenLoading } from "@/components";
import { clerk } from "@/lib/auth";
import { cloudEnv } from "@/lib/env";

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

function CloudRoute() {
	const navigate = useNavigate();
	return (
		<ClerkProvider
			Clerk={clerk}
			appearance={{
				baseTheme: dark,
				variables: {
					colorPrimary: "hsl(var(--primary))",
					colorPrimaryForeground: "hsl(var(--primary-foreground))",
					colorTextOnPrimaryBackground:
						"hsl(var(--primary-foreground))",
					colorBackground: "hsl(var(--background))",
					colorInput: "hsl(var(--input))",
					colorText: "hsl(var(--text))",
					colorTextSecondary: "hsl(var(--muted-foreground))",
					borderRadius: "var(--radius)",
					colorModalBackdrop: "rgb(0 0 0 / 0.8)",
				},
			}}
			publishableKey={cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY}
			routerPush={(to) => navigate({ to })}
			routerReplace={(to) => navigate({ to, replace: true })}
			taskUrls={{
				"choose-organization": "/onboarding/choose-organization",
			}}
		>
			<Outlet />
			{import.meta.env.DEV ? (
				<TanStackRouterDevtools position="bottom-right" />
			) : null}
		</ClerkProvider>
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
	component: match(__APP_TYPE__)
		.with("cloud", () => CloudRoute)
		.otherwise(() => RootRoute),
	pendingComponent: FullscreenLoading,
});
