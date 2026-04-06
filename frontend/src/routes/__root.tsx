import type { Clerk } from "@clerk/clerk-js";
import type { QueryClient } from "@tanstack/react-query";
import {
	Outlet,
	createRootRouteWithContext,
	useNavigate,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { type PropsWithChildren, Suspense, lazy } from "react";
import { match } from "ts-pattern";
import type {
	CloudContext,
	CloudNamespaceContext,
	EngineContext,
	EngineNamespaceContext,
	OrganizationContext,
	ProjectContext,
} from "@/app/data-providers/cache";
import { FullscreenLoading } from "@/components";
import { clerkPromise } from "@/lib/auth";
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

const LazyClerkProvider = lazy(() =>
	Promise.all([
		import("@clerk/clerk-react"),
		import("@clerk/themes"),
		clerkPromise,
	]).then(([{ ClerkProvider }, { dark }, clerk]) => ({
		default: ({ children, navigatePush, navigateReplace }: PropsWithChildren<{ navigatePush: (to: string) => void; navigateReplace: (to: string) => void }>) => (
			<ClerkProvider
				Clerk={clerk}
				appearance={{
					baseTheme: dark,
					variables: {
						colorPrimary: "hsl(var(--primary))",
						colorPrimaryForeground: "hsl(var(--primary-foreground))",
						colorTextOnPrimaryBackground: "hsl(var(--primary-foreground))",
						colorBackground: "hsl(var(--background))",
						colorInput: "hsl(var(--input))",
						colorText: "hsl(var(--text))",
						colorTextSecondary: "hsl(var(--muted-foreground))",
						borderRadius: "var(--radius)",
						colorModalBackdrop: "rgb(0 0 0 / 0.8)",
					},
				}}
				publishableKey={cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY}
				routerPush={(to: string) => navigatePush(to)}
				routerReplace={(to: string) => navigateReplace(to)}
				signInUrl="/login"
				signUpUrl="/join"
				signInForceRedirectUrl="/sso-callback"
				signUpForceRedirectUrl="/sso-callback"
				taskUrls={{
					"choose-organization": "/onboarding/choose-organization",
				}}
			>
				{children}
			</ClerkProvider>
		),
	})),
);

function CloudRoute() {
	const navigate = useNavigate();
	return (
		<Suspense fallback={<FullscreenLoading />}>
			<LazyClerkProvider
				navigatePush={(to) => navigate({ to })}
				navigateReplace={(to) => navigate({ to, replace: true })}
			>
				<Outlet />
				{import.meta.env.DEV ? (
					<TanStackRouterDevtools position="bottom-right" />
				) : null}
			</LazyClerkProvider>
		</Suspense>
	);
}

interface RootRouteContext {
	/**
	 * Only available in cloud mode
	 */
	clerk: Clerk;
	queryClient: QueryClient;
	getOrCreateCloudContext: (clerk: Clerk) => CloudContext;
	getOrCreateEngineContext: (
		engineToken: (() => string) | string | (() => Promise<string>),
	) => EngineContext;
	getOrCreateOrganizationContext: (
		parent: CloudContext,
		organization: string,
	) => OrganizationContext;
	getOrCreateProjectContext: (
		parent: CloudContext & OrganizationContext,
		organization: string,
		project: string,
	) => ProjectContext;
	getOrCreateCloudNamespaceContext: (
		parent: CloudContext & OrganizationContext & ProjectContext,
		namespace: string,
		engineNamespaceName: string,
		engineNamespaceId: string,
	) => CloudNamespaceContext;
	getOrCreateEngineNamespaceContext: (
		parent: EngineContext,
		namespace: string,
	) => EngineNamespaceContext;
}

export const Route = createRootRouteWithContext<RootRouteContext>()({
	component: match(__APP_TYPE__)
		.with("cloud", () => CloudRoute)
		.otherwise(() => RootRoute),
	pendingComponent: FullscreenLoading,
});
